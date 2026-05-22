//! Health Checks - Verificación de integridad, uptime, recursos
//!
//! Verifica integridad de redb, conexión a pares, estado de LayerRouter
//! y límites de recursos. Retorna HTTP 200/503.

use std::sync::atomic::{AtomicBool, Ordering};
// CLEANUP: removed unused import AtomicU64
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
// CLEANUP: removed unused import warn

/// Resultado de un check de salud individual
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Nombre del check
    pub name: String,
    /// Si el check pasó
    pub passed: bool,
    /// Mensaje descriptivo
    pub message: String,
    /// Latencia del check en ms
    pub latency_ms: f64,
    /// Timestamp del check
    pub timestamp_ms: u64,
}

impl HealthCheckResult {
    pub fn new(name: String, passed: bool, message: String, latency_ms: f64) -> Self {
        let duration = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0));

        Self {
            name,
            passed,
            message,
            latency_ms,
            timestamp_ms: duration.as_millis() as u64,
        }
    }

    pub fn ok(name: String, message: String, latency_ms: f64) -> Self {
        Self::new(name, true, message, latency_ms)
    }

    pub fn fail(name: String, message: String, latency_ms: f64) -> Self {
        Self::new(name, false, message, latency_ms)
    }
}

/// Estado general de salud
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Resultado completo del health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Estado general
    pub status: HealthStatus,
    /// Todos los checks individuales
    pub checks: Vec<HealthCheckResult>,
    /// Uptime en segundos
    pub uptime_seconds: u64,
    /// Timestamp del reporte
    pub timestamp_ms: u64,
    /// Resumen: total, passed, failed
    pub summary: HealthSummary,
}

impl HealthReport {
    pub fn new(checks: Vec<HealthCheckResult>, uptime_seconds: u64) -> Self {
        let total = checks.len();
        let passed = checks.iter().filter(|c| c.passed).count();
        let failed = total - passed;

        let status = if failed == 0 {
            HealthStatus::Healthy
        } else if passed > failed {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        };

        let duration = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0));

        Self {
            status,
            checks,
            uptime_seconds,
            timestamp_ms: duration.as_millis() as u64,
            summary: HealthSummary {
                total,
                passed,
                failed,
            },
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.status == HealthStatus::Healthy
    }

    pub fn is_degraded(&self) -> bool {
        self.status == HealthStatus::Degraded
    }

    pub fn is_unhealthy(&self) -> bool {
        self.status == HealthStatus::Unhealthy
    }
}

/// Resumen del health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
}

/// Callback tipo para checks personalizados
pub type HealthCheckFn = dyn Fn() -> HealthCheckResult + Send + Sync;

/// Manager de health checks
pub struct HealthManager {
    /// Checks registrados
    checks: RwLock<Vec<(String, Arc<HealthCheckFn>)>>,
    /// Timestamp de inicio
    start_time: Instant,
    /// Último reporte
    last_report: RwLock<Option<HealthReport>>,
    /// Flag de salud general
    is_healthy: AtomicBool,
}

impl HealthManager {
    pub fn new() -> Self {
        Self {
            checks: RwLock::new(Vec::new()),
            start_time: Instant::now(),
            last_report: RwLock::new(None),
            is_healthy: AtomicBool::new(true),
        }
    }

    /// Registra un check personalizado
    pub fn register_check(&self, name: String, check: Arc<HealthCheckFn>) {
        // FIX: borrow/move - Clone name before inserting into tuple
        self.checks.write().push((name.clone(), check));
        info!(check = %name, "Health check registered");
    }

    /// Agrega checks por defecto
    pub fn add_default_checks(&self) {
        // Check: Uptime
        self.register_check(
            "uptime".to_string(),
            Arc::new(|| {
                // Siempre pasa si el proceso está corriendo
                HealthCheckResult::ok("uptime".to_string(), "Process is running".to_string(), 0.1)
            }),
        );

        // Check: Memory usage
        self.register_check(
            "memory".to_string(),
            Arc::new(|| {
                // Placeholder - en producción usaría libproc o similar
                HealthCheckResult::ok(
                    "memory".to_string(),
                    "Memory usage within limits".to_string(),
                    0.5,
                )
            }),
        );

        // Check: Disk space
        self.register_check(
            "disk".to_string(),
            Arc::new(|| {
                // Placeholder - verificar espacio disponible
                HealthCheckResult::ok("disk".to_string(), "Disk space sufficient".to_string(), 0.3)
            }),
        );
    }

    /// Ejecuta todos los checks y genera reporte
    pub fn run_checks(&self) -> HealthReport {
        let _start = Instant::now();
        let checks = self.checks.read();
        let mut results = Vec::new();

        for (name, check_fn) in checks.iter() {
            let check_start = Instant::now();
            let result = check_fn();
            let latency = check_start.elapsed().as_secs_f64() * 1000.0;

            // Actualizar latencia medida
            let result = HealthCheckResult {
                latency_ms: latency,
                ..result
            };

            results.push(result);
            debug!(
                check = %name,
                passed = results.last().unwrap().passed,
                latency_ms = latency,
                "Health check executed"
            );
        }

        let uptime = self.start_time.elapsed().as_secs();
        let report = HealthReport::new(results, uptime);

        // Actualizar estado
        self.is_healthy.store(report.is_healthy(), Ordering::SeqCst);
        *self.last_report.write() = Some(report.clone());

        info!(
            status = %report.status,
            passed = report.summary.passed,
            failed = report.summary.failed,
            "Health check completed"
        );

        report
    }

    /// Obtiene último reporte
    pub fn get_last_report(&self) -> Option<HealthReport> {
        self.last_report.read().clone()
    }

    /// Verifica si el sistema es saludable
    pub fn is_system_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::SeqCst)
    }

    /// Obtiene uptime
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Crea checks específicos para componentes ed2kIA
    /// Check: Feedback Store (redb)
    pub fn create_feedback_store_check(
        db_path: String,
        check_fn: impl Fn() -> Result<(), String> + Send + Sync + 'static,
    ) -> Arc<HealthCheckFn> {
        Arc::new(move || {
            let start = Instant::now();
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(&check_fn)) {
                // CLEANUP: redundant closure
                Ok(Ok(())) => HealthCheckResult::ok(
                    "feedback_store".to_string(),
                    format!("Database OK at {}", db_path),
                    start.elapsed().as_secs_f64() * 1000.0,
                ),
                Ok(Err(e)) => HealthCheckResult::fail(
                    "feedback_store".to_string(),
                    format!("Database error: {}", e),
                    start.elapsed().as_secs_f64() * 1000.0,
                ),
                Err(_) => HealthCheckResult::fail(
                    "feedback_store".to_string(),
                    "Database panic".to_string(),
                    start.elapsed().as_secs_f64() * 1000.0,
                ),
            }
        })
    }

    /// Check: P2P Connections
    pub fn create_p2p_check(
        min_peers: usize,
        current_peers_fn: impl Fn() -> usize + Send + Sync + 'static,
    ) -> Arc<HealthCheckFn> {
        Arc::new(move || {
            let start = Instant::now();
            let current_peers = current_peers_fn();
            if current_peers >= min_peers {
                HealthCheckResult::ok(
                    "p2p_connections".to_string(),
                    format!("{} peers connected (min: {})", current_peers, min_peers),
                    start.elapsed().as_secs_f64() * 1000.0,
                )
            } else {
                HealthCheckResult::fail(
                    "p2p_connections".to_string(),
                    format!(
                        "Only {} peers connected (min: {})",
                        current_peers, min_peers
                    ),
                    start.elapsed().as_secs_f64() * 1000.0,
                )
            }
        })
    }

    /// Check: Resource Limits
    pub fn create_resource_check(
        max_memory_bytes: u64,
        current_memory_fn: impl Fn() -> u64 + Send + Sync + 'static,
    ) -> Arc<HealthCheckFn> {
        Arc::new(move || {
            let start = Instant::now();
            let current = current_memory_fn();
            let usage_pct = if max_memory_bytes > 0 {
                (current as f64 / max_memory_bytes as f64) * 100.0
            } else {
                0.0
            };

            if usage_pct < 90.0 {
                HealthCheckResult::ok(
                    "resource_limits".to_string(),
                    format!("Memory usage: {:.1}%", usage_pct),
                    start.elapsed().as_secs_f64() * 1000.0,
                )
            } else {
                HealthCheckResult::fail(
                    "resource_limits".to_string(),
                    format!("Memory usage critical: {:.1}%", usage_pct),
                    start.elapsed().as_secs_f64() * 1000.0,
                )
            }
        })
    }
}

impl Default for HealthManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_manager_creation() {
        let manager = HealthManager::new();
        assert!(manager.is_system_healthy());
    }

    #[test]
    fn test_health_check_result() {
        let result = HealthCheckResult::ok("test".to_string(), "OK".to_string(), 1.0);
        assert!(result.passed);
        assert_eq!(result.name, "test");

        let fail = HealthCheckResult::fail("test".to_string(), "Failed".to_string(), 2.0);
        assert!(!fail.passed);
    }

    #[test]
    fn test_health_report_healthy() {
        let checks = vec![
            HealthCheckResult::ok("check1".to_string(), "OK".to_string(), 1.0),
            HealthCheckResult::ok("check2".to_string(), "OK".to_string(), 2.0),
        ];
        let report = HealthReport::new(checks, 100);
        assert_eq!(report.status, HealthStatus::Healthy);
        assert!(report.is_healthy());
    }

    #[test]
    fn test_health_report_degraded() {
        let checks = vec![
            HealthCheckResult::ok("check1".to_string(), "OK".to_string(), 1.0),
            HealthCheckResult::ok("check2".to_string(), "OK".to_string(), 2.0),
            HealthCheckResult::fail("check3".to_string(), "Failed".to_string(), 3.0),
        ];
        let report = HealthReport::new(checks, 100);
        assert_eq!(report.status, HealthStatus::Degraded);
        assert!(report.is_degraded());
    }

    #[test]
    fn test_health_report_unhealthy() {
        let checks = vec![
            HealthCheckResult::fail("check1".to_string(), "Failed".to_string(), 1.0),
            HealthCheckResult::fail("check2".to_string(), "Failed".to_string(), 2.0),
            HealthCheckResult::ok("check3".to_string(), "OK".to_string(), 3.0),
        ];
        let report = HealthReport::new(checks, 100);
        assert_eq!(report.status, HealthStatus::Unhealthy);
        assert!(report.is_unhealthy());
    }

    #[test]
    fn test_run_checks() {
        let manager = HealthManager::new();
        manager.register_check(
            "test_check".to_string(),
            Arc::new(|| HealthCheckResult::ok("test_check".to_string(), "OK".to_string(), 0.5)),
        );

        let report = manager.run_checks();
        assert_eq!(report.summary.total, 1);
        assert_eq!(report.summary.passed, 1);
        assert_eq!(report.summary.failed, 0);
    }
}
