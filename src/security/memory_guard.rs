//! Memory Guard - Límites de memoria, sandbox de E/S y detección de escapes
//!
//! Proporciona:
//! - Límites de memoria por sandbox
//! - Detección de intentos de escape (lecturas fuera de rango)
//! - Tracking de allocations para prevenir OOM
//! - Validación de buffers de salida

use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, info, warn};

/// Guardián de memoria para un sandbox WASM
pub struct MemoryGuard {
    /// Límite máximo de memoria en bytes
    limit_bytes: u64,
    /// Uso actual de memoria (atómico para thread-safety)
    current_usage: AtomicU64,
    /// Uso pico de memoria
    peak_usage: AtomicU64,
    /// Número total de allocations
    allocation_count: AtomicU64,
    /// Número total de deallocations
    deallocation_count: AtomicU64,
    /// Lista de allocations activas (protegida por mutex)
    active_allocations: Mutex<Vec<AllocationRecord>>,
    /// Escapes detectados
    escape_count: AtomicU64,
}

/// Registro de una allocation individual
#[derive(Debug, Clone)]
struct AllocationRecord {
    /// Tamaño en bytes
    size_bytes: usize,
    /// Timestamp de creación (ns desde epoch)
    created_at: u128,
    /// Etiqueta opcional para debugging
    label: Option<String>,
}

impl MemoryGuard {
    /// Crea un nuevo MemoryGuard con el límite especificado
    pub fn new(limit_bytes: u64) -> Self {
        info!("MemoryGuard created: limit={}MB", limit_bytes / (1024 * 1024));
        Self {
            limit_bytes,
            current_usage: AtomicU64::new(0),
            peak_usage: AtomicU64::new(0),
            allocation_count: AtomicU64::new(0),
            deallocation_count: AtomicU64::new(0),
            active_allocations: Mutex::new(Vec::new()),
            escape_count: AtomicU64::new(0),
        }
    }

    /// Verifica si se puede realizar una allocation del tamaño especificado
    pub fn check_before_alloc(&self, requested_bytes: usize) -> Result<()> {
        let requested = requested_bytes as u64;
        let current = self.current_usage.load(Ordering::Relaxed);

        if current + requested > self.limit_bytes {
            self.escape_count.fetch_add(1, Ordering::Relaxed);
            return Err(anyhow!(
                "Memory allocation would exceed limit: current={}MB, requested={}KB, limit={}MB",
                current / (1024 * 1024),
                requested_bytes / 1024,
                self.limit_bytes / (1024 * 1024)
            ));
        }

        debug!(
            "Allocation check passed: {}KB available of {}MB",
            requested_bytes / 1024,
            (self.limit_bytes - current) / (1024 * 1024)
        );
        Ok(())
    }

    /// Registra una allocation de memoria
    pub fn record_alloc(&self, size_bytes: usize, label: Option<&str>) -> Result<()> {
        self.check_before_alloc(size_bytes)?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        let record = AllocationRecord {
            size_bytes,
            created_at: now,
            label: label.map(|s| s.to_string()),
        };

        {
            let mut allocs = self.active_allocations.lock();
            allocs.push(record);
        }

        let new_usage = self.current_usage.fetch_add(size_bytes as u64, Ordering::AcqRel)
            + size_bytes as u64;

        // Actualiza pico de memoria
        self.update_peak(new_usage);

        self.allocation_count.fetch_add(1, Ordering::Relaxed);

        debug!(
            "Allocated {}KB [{}]: total={}MB, peak={}MB",
            size_bytes / 1024,
            label.unwrap_or("unknown"),
            new_usage / (1024 * 1024),
            self.peak_usage.load(Ordering::Relaxed) / (1024 * 1024)
        );

        Ok(())
    }

    /// Registra una deallocation de memoria
    pub fn record_dealloc(&self, size_bytes: usize) {
        let previous = self.current_usage.fetch_sub(size_bytes as u64, Ordering::AcqRel);
        self.deallocation_count.fetch_add(1, Ordering::Relaxed);

        debug!(
            "Deallocated {}KB: remaining={}MB",
            size_bytes / 1024,
            (previous - size_bytes as u64) / (1024 * 1024)
        );
    }

    /// Actualiza el pico de memoria si el nuevo valor es mayor
    fn update_peak(&self, new_usage: u64) {
        let mut peak = self.peak_usage.load(Ordering::Relaxed);
        while new_usage > peak {
            match self.peak_usage.compare_exchange_weak(
                peak,
                new_usage,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(actual) => peak = actual,
            }
        }
    }

    /// Obtiene el uso actual de memoria en bytes
    pub fn current_usage(&self) -> u64 {
        self.current_usage.load(Ordering::Relaxed)
    }

    /// Obtiene el pico de memoria en bytes
    pub fn peak_usage(&self) -> u64 {
        self.peak_usage.load(Ordering::Relaxed)
    }

    /// Obtiene el límite de memoria en bytes
    pub fn limit(&self) -> u64 {
        self.limit_bytes
    }

    /// Obtiene el porcentaje de uso de memoria
    pub fn usage_percent(&self) -> f64 {
        if self.limit_bytes == 0 {
            return 100.0;
        }
        (self.current_usage.load(Ordering::Relaxed) as f64 / self.limit_bytes as f64) * 100.0
    }

    /// Valida que un buffer de salida no contenga patrones sospechosos
    pub fn validate_output(&self, output: &[u8]) -> Result<()> {
        // Verifica tamaño razonable (no más del 10% del límite)
        let max_reasonable = (self.limit_bytes as f64 * 0.10) as u64;
        if output.len() as u64 > max_reasonable {
            self.escape_count.fetch_add(1, Ordering::Relaxed);
            warn!(
                "Output size {}KB exceeds 10% of memory limit {}MB",
                output.len() / 1024,
                self.limit_bytes / (1024 * 1024)
            );
            return Err(anyhow!(
                "Output buffer too large: {}KB > {}KB (10% limit)",
                output.len() / 1024,
                max_reasonable / 1024
            ));
        }

        // Verifica patrones de escape comunes
        if self.detect_memory_leak_pattern(output) {
            self.escape_count.fetch_add(1, Ordering::Relaxed);
            warn!("Suspicious pattern detected in output buffer");
            return Err(anyhow!("Suspicious memory pattern detected in output"));
        }

        debug!("Output validation passed: {}KB", output.len() / 1024);
        Ok(())
    }

    /// Detecta patrones sospechosos que podrían indicar memory leaks o escapes
    fn detect_memory_leak_pattern(&self, data: &[u8]) -> bool {
        if data.is_empty() {
            return false;
        }

        // Patrón 1: Todos los bytes iguales (posible uninitialized memory)
        let first = data[0];
        if data.iter().all(|&b| b == first) && (first == 0 || first == 0xFF || first == 0xAB) {
            return true;
        }

        // Patrón 2: Secuencia de punteros nulos (posible heap spray)
        if data.len() >= 8 {
            let null_count = data.iter().filter(|&&b| b == 0).count();
            if null_count as f64 / data.len() as f64 > 0.95 {
                return true;
            }
        }

        // Patrón 3: Repetición de bloques de 8 bytes (posible address leak)
        if data.len() >= 16 && data.len().is_multiple_of(8) {
            let first_block: [u8; 8] = data[..8].try_into().unwrap();
            let repetitions = data.chunks(8).filter(|c| *c == first_block.as_slice()).count();
            if repetitions * 8 > data.len() / 2 {
                return true;
            }
        }

        false
    }

    /// Resetea el guardián de memoria (para pruebas o reinicio de sandbox)
    pub fn reset(&self) {
        self.current_usage.store(0, Ordering::Relaxed);
        self.peak_usage.store(0, Ordering::Relaxed);
        self.allocation_count.store(0, Ordering::Relaxed);
        self.deallocation_count.store(0, Ordering::Relaxed);
        self.escape_count.store(0, Ordering::Relaxed);
        *self.active_allocations.lock() = Vec::new();
        info!("MemoryGuard reset");
    }

    /// Obtiene estadísticas completas del guardián
    pub fn get_stats(&self) -> MemoryStats {
        MemoryStats {
            limit_bytes: self.limit_bytes,
            current_usage: self.current_usage.load(Ordering::Relaxed),
            peak_usage: self.peak_usage.load(Ordering::Relaxed),
            allocation_count: self.allocation_count.load(Ordering::Relaxed),
            deallocation_count: self.deallocation_count.load(Ordering::Relaxed),
            active_allocations: self.active_allocations.lock().len(),
            escape_count: self.escape_count.load(Ordering::Relaxed),
            usage_percent: self.usage_percent(),
        }
    }

    /// Obtiene el número de escapes detectados
    pub fn escape_count(&self) -> u64 {
        self.escape_count.load(Ordering::Relaxed)
    }

    /// Verifica si se ha superado un umbral de uso (para alertas tempranas)
    pub fn is_over_threshold(&self, threshold_percent: f64) -> bool {
        self.usage_percent() > threshold_percent
    }
}

/// Estadísticas de memoria
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub limit_bytes: u64,
    pub current_usage: u64,
    pub peak_usage: u64,
    pub allocation_count: u64,
    pub deallocation_count: u64,
    pub active_allocations: usize,
    pub escape_count: u64,
    pub usage_percent: f64,
}

impl MemoryStats {
    pub fn available_bytes(&self) -> u64 {
        self.limit_bytes.saturating_sub(self.current_usage)
    }

    pub fn available_mb(&self) -> f64 {
        self.available_bytes() as f64 / (1024.0 * 1024.0)
    }

    pub fn leak_suspected(&self) -> bool {
        // Si hay muchas más allocations que deallocations
        let diff = self.allocation_count as isize - self.deallocation_count as isize;
        diff > 100 // Umbral arbitrario para detección temprana
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_guard_creation() {
        let guard = MemoryGuard::new(1024 * 1024); // 1MB
        assert_eq!(guard.limit(), 1024 * 1024);
        assert_eq!(guard.current_usage(), 0);
        assert_eq!(guard.usage_percent(), 0.0);
    }

    #[test]
    fn test_alloc_within_limit() {
        let guard = MemoryGuard::new(1024); // 1KB
        assert!(guard.check_before_alloc(512).is_ok());
    }

    #[test]
    fn test_alloc_exceeds_limit() {
        let guard = MemoryGuard::new(1024); // 1KB
        assert!(guard.check_before_alloc(2048).is_err());
    }

    #[test]
    fn test_record_alloc_and_dealloc() {
        let guard = MemoryGuard::new(4096);
        guard.record_alloc(1024, Some("test")).unwrap();
        assert_eq!(guard.current_usage(), 1024);

        guard.record_dealloc(1024);
        assert_eq!(guard.current_usage(), 0);
    }

    #[test]
    fn test_peak_usage_tracking() {
        let guard = MemoryGuard::new(4096);
        guard.record_alloc(2048, Some("peak_test")).unwrap();
        guard.record_dealloc(2048);

        assert_eq!(guard.peak_usage(), 2048);
        assert_eq!(guard.current_usage(), 0);
    }

    #[test]
    fn test_escape_detection_all_zeros() {
        let guard = MemoryGuard::new(1024 * 1024);
        let zeros = vec![0u8; 64];
        assert!(guard.detect_memory_leak_pattern(&zeros));
    }

    #[test]
    fn test_escape_detection_normal_data() {
        let guard = MemoryGuard::new(1024 * 1024);
        let normal_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        assert!(!guard.detect_memory_leak_pattern(&normal_data));
    }

    #[test]
    fn test_reset() {
        let guard = MemoryGuard::new(4096);
        guard.record_alloc(1024, Some("reset_test")).unwrap();
        guard.reset();
        assert_eq!(guard.current_usage(), 0);
        assert_eq!(guard.peak_usage(), 0);
    }

    #[test]
    fn test_memory_stats() {
        let stats = MemoryStats {
            limit_bytes: 1024 * 1024,
            current_usage: 512 * 1024,
            peak_usage: 768 * 1024,
            allocation_count: 10,
            deallocation_count: 3,
            active_allocations: 7,
            escape_count: 0,
            usage_percent: 50.0,
        };

        assert_eq!(stats.available_bytes(), 512 * 1024);
        assert!((stats.available_mb() - 0.5).abs() < 0.01);
        assert!(!stats.leak_suspected());
    }

    #[test]
    fn test_leak_suspected() {
        let stats = MemoryStats {
            limit_bytes: 1024 * 1024,
            current_usage: 0,
            peak_usage: 0,
            allocation_count: 200,
            deallocation_count: 50,
            active_allocations: 150,
            escape_count: 0,
            usage_percent: 0.0,
        };

        assert!(stats.leak_suspected());
    }
}
