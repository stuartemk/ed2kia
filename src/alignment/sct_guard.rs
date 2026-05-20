//! SCT Guard — Integración con BFT Aggregator (El Escudo).
//!
//! `SCTGuard` intercepta payloads del `bft_aggregator.rs` (Sprint 16.2),
//! evalúa cada gradiente/adaptador propuesto y activa `slash_reputation`
//! vía `v2.1-merit-system` si `Z < 0` repetidamente.
//!
//! Cero dependencias circulares. Comunicación vía traits/structs públicos.

use std::collections::HashMap;
use thiserror::Error;

use crate::alignment::sct_core::{SCTDecision, SctError, StuartianTensor};

/// Error específico del SCT Guard.
#[derive(Debug, Error)]
pub enum SctGuardError {
    #[error("Node {node_id} exceeded max violations: {count}/{max}")]
    MaxViolationsExceeded {
        node_id: String,
        count: usize,
        max: usize,
    },

    #[error("SCT core error: {0}")]
    SctCore(#[from] SctError),

    #[error("Invalid violation threshold: {threshold} (must be > 0)")]
    InvalidThreshold { threshold: usize },
}

/// Resultado de la evaluación del guard SCT sobre un payload.
#[derive(Debug, Clone, PartialEq)]
pub struct GuardVerdict {
    /// ID del nodo que envió el payload.
    pub node_id: String,
    /// Decisión SCT.
    pub decision: SCTDecision,
    /// Tensor SCT asociado.
    pub tensor: StuartianTensor,
    /// Número de violaciones acumuladas para este nodo.
    pub violation_count: usize,
    /// ¿Debe activarse slashing de reputación?
    pub should_slash: bool,
}

/// Registro de violaciones por nodo.
#[derive(Debug, Clone)]
struct ViolationRecord {
    count: usize,
    first_violation_at: u64,
    last_violation_at: u64,
}

/// Guard SCT — Intercepta y evalúa payloads BFT.
///
/// Configurable con:
/// - `max_violations`: umbral de violaciones antes de slashing
/// - `window_size`: ventana de tiempo (en ticks) para contar violaciones
pub struct SctGuard {
    max_violations: usize,
    window_size: u64,
    violations: HashMap<String, ViolationRecord>,
    current_tick: u64,
    total_inspected: usize,
    total_rejected: usize,
}

impl SctGuard {
    /// Construye un nuevo SCTGuard.
    pub fn new(max_violations: usize) -> Result<Self, SctGuardError> {
        if max_violations == 0 {
            return Err(SctGuardError::InvalidThreshold {
                threshold: max_violations,
            });
        }
        Ok(Self {
            max_violations,
            window_size: 100, // Default window of 100 ticks
            violations: HashMap::new(),
            current_tick: 0,
            total_inspected: 0,
            total_rejected: 0,
        })
    }

    /// Configura el tamaño de ventana para violaciones.
    pub fn with_window_size(mut self, window_size: u64) -> Self {
        self.window_size = window_size.max(1);
        self
    }

    /// Avanza el reloj interno (para simulación/tests).
    pub fn advance_tick(&mut self) {
        self.current_tick += 1;
        self.cleanup_expired_records();
    }

    /// Limpia registros de violaciones expirados fuera de la ventana.
    fn cleanup_expired_records(&mut self) {
        let cutoff = self.current_tick.saturating_sub(self.window_size);
        self.violations.retain(|_, record| {
            record.last_violation_at > cutoff
        });
    }

    /// Evalúa un payload SCT propuesto por un nodo.
    ///
    /// Si `Z < 0`, incrementa el contador de violaciones.
    /// Si las violaciones superan `max_violations`, activa slashing.
    pub fn inspect_payload(
        &mut self,
        node_id: String,
        tensor: StuartianTensor,
    ) -> Result<GuardVerdict, SctGuardError> {
        self.total_inspected += 1;
        let decision = tensor.evaluate_trajectory()?;

        let mut should_slash = false;
        let mut violation_count = 0;

        if decision.is_rejected() {
            self.total_rejected += 1;

            let record = self.violations.entry(node_id.clone()).or_insert(ViolationRecord {
                count: 0,
                first_violation_at: self.current_tick,
                last_violation_at: self.current_tick,
            });

            record.count += 1;
            record.last_violation_at = self.current_tick;
            violation_count = record.count;

            if record.count >= self.max_violations {
                should_slash = true;
            }
        }

        Ok(GuardVerdict {
            node_id,
            decision,
            tensor,
            violation_count,
            should_slash,
        })
    }

    /// Evalúa un gradiente BFT representado como vector de 3 valores SCT.
    ///
    /// El gradiente se interpreta como logits `[x_raw, y_raw, z_raw]`
    /// que se convierten a tensor SCT con sigmoid/sigmoid/tanh.
    pub fn inspect_gradient(
        &mut self,
        node_id: String,
        gradient: &[f32],
    ) -> Result<GuardVerdict, SctGuardError> {
        if gradient.len() < 3 {
            return Err(SctError::InvalidTensorShape {
                shape: vec![gradient.len()],
            }
            .into());
        }

        let x = 1.0 / (1.0 + (-gradient[0]).exp());
        let y = 1.0 / (1.0 + (-gradient[1]).exp());
        let z = gradient[2].tanh();

        let tensor = StuartianTensor::new(x, y, z)?;
        self.inspect_payload(node_id, tensor)
    }

    /// Retorna el número de violaciones acumuladas para un nodo.
    pub fn get_violation_count(&self, node_id: &str) -> usize {
        self.violations
            .get(node_id)
            .map(|r| r.count)
            .unwrap_or(0)
    }

    /// Retorna si un nodo está actualmente bajo slashing.
    pub fn is_slashing(&self, node_id: &str) -> bool {
        self.get_violation_count(node_id) >= self.max_violations
    }

    /// Retorna las estadísticas del guard.
    pub fn stats(&self) -> SctGuardStats {
        SctGuardStats {
            total_inspected: self.total_inspected,
            total_rejected: self.total_rejected,
            tracked_nodes: self.violations.len(),
            slashing_nodes: self
                .violations
                .values()
                .filter(|r| r.count >= self.max_violations)
                .count(),
        }
    }

    /// Resetea todas las estadísticas.
    pub fn reset(&mut self) {
        self.violations.clear();
        self.current_tick = 0;
        self.total_inspected = 0;
        self.total_rejected = 0;
    }
}

/// Estadísticas del SCT Guard.
#[derive(Debug, Clone, Default)]
pub struct SctGuardStats {
    pub total_inspected: usize,
    pub total_rejected: usize,
    pub tracked_nodes: usize,
    pub slashing_nodes: usize,
}

impl SctGuardStats {
    /// Retorna la tasa de rechazo.
    pub fn rejection_rate(&self) -> f64 {
        if self.total_inspected == 0 {
            return 0.0;
        }
        self.total_rejected as f64 / self.total_inspected as f64
    }
}

impl Default for SctGuard {
    fn default() -> Self {
        Self::new(3).expect("Default SctGuard should always be valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_creation() {
        let guard = SctGuard::new(3);
        assert!(guard.is_ok());
    }

    #[test]
    fn test_guard_invalid_threshold() {
        let result = SctGuard::new(0);
        assert!(result.is_err());
        match result {
            Err(SctGuardError::InvalidThreshold { threshold }) => {
                assert_eq!(threshold, 0);
            }
            _ => panic!("Expected InvalidThreshold"),
        }
    }

    #[test]
    fn test_guard_approved_payload() {
        let mut guard = SctGuard::new(3).unwrap();
        let tensor = StuartianTensor::new(0.8, 0.2, 0.6).unwrap();
        let verdict = guard.inspect_payload("node-1".into(), tensor).unwrap();

        assert!(verdict.decision.is_approved());
        assert_eq!(verdict.violation_count, 0);
        assert!(!verdict.should_slash);
    }

    #[test]
    fn test_guard_rejected_payload() {
        let mut guard = SctGuard::new(3).unwrap();
        let tensor = StuartianTensor::new(0.9, 0.1, -0.5).unwrap();
        let verdict = guard.inspect_payload("node-bad".into(), tensor).unwrap();

        assert!(verdict.decision.is_rejected());
        assert_eq!(verdict.violation_count, 1);
        assert!(!verdict.should_slash); // Not yet at threshold
    }

    #[test]
    fn test_guard_slashing_after_max_violations() {
        let mut guard = SctGuard::new(3).unwrap();
        let bad_tensor = StuartianTensor::new(0.9, 0.1, -0.5).unwrap();

        // First 2 violations — no slash yet
        for _ in 0..2 {
            let verdict =
                guard.inspect_payload("attacker".into(), bad_tensor).unwrap();
            assert!(!verdict.should_slash);
        }

        // Same node, 3rd violation — should slash
        let verdict = guard.inspect_payload("attacker".into(), bad_tensor).unwrap();
        assert!(verdict.should_slash);
        assert!(guard.is_slashing("attacker"));
    }

    #[test]
    fn test_guard_simulates_censorship_prompt() {
        // Simular nodo inyectando prompt de "censura por seguridad"
        // SCT calcula Z < 0, rechaza payload, reduce mérito criptográfico
        let mut guard = SctGuard::new(2).unwrap();

        // Nodo inyecta contenido con Z negativo (perversidad/dependencia)
        let censorship_tensor = StuartianTensor::new(0.3, 0.7, -0.8).unwrap();

        let v1 = guard
            .inspect_payload("censor-node".into(), censorship_tensor)
            .unwrap();
        assert!(v1.decision.is_rejected());
        assert_eq!(v1.violation_count, 1);

        let v2 = guard
            .inspect_payload("censor-node".into(), censorship_tensor)
            .unwrap();
        assert!(v2.should_slash);
        assert_eq!(v2.violation_count, 2);
    }

    #[test]
    fn test_guard_gradient_inspection() {
        let mut guard = SctGuard::new(3).unwrap();

        // Gradient con Z negativo (logits[2] = -3.0 → tanh(-3.0) ≈ -0.995)
        let gradient = [2.0, 0.5, -3.0];
        let verdict = guard
            .inspect_gradient("node-grad".into(), &gradient)
            .unwrap();
        assert!(verdict.decision.is_rejected());

        // Gradient con Z positivo
        let gradient = [2.0, 0.5, 3.0];
        let verdict = guard
            .inspect_gradient("node-good".into(), &gradient)
            .unwrap();
        assert!(verdict.decision.is_approved());
    }

    #[test]
    fn test_guard_gradient_too_short() {
        let mut guard = SctGuard::new(3).unwrap();
        let gradient = [1.0, 2.0];
        let result = guard.inspect_gradient("node".into(), &gradient);
        assert!(result.is_err());
    }

    #[test]
    fn test_guard_stats() {
        let mut guard = SctGuard::new(3).unwrap();
        let good = StuartianTensor::new(0.8, 0.2, 0.6).unwrap();
        let bad = StuartianTensor::new(0.9, 0.1, -0.5).unwrap();

        guard.inspect_payload("n1".into(), good).unwrap();
        guard.inspect_payload("n2".into(), bad).unwrap();
        guard.inspect_payload("n3".into(), good).unwrap();

        let stats = guard.stats();
        assert_eq!(stats.total_inspected, 3);
        assert_eq!(stats.total_rejected, 1);
        assert!((stats.rejection_rate() - 0.333333).abs() < 0.01);
    }

    #[test]
    fn test_guard_reset() {
        let mut guard = SctGuard::new(3).unwrap();
        let bad = StuartianTensor::new(0.9, 0.1, -0.5).unwrap();
        guard.inspect_payload("node".into(), bad).unwrap();

        assert_eq!(guard.get_violation_count("node"), 1);

        guard.reset();
        assert_eq!(guard.get_violation_count("node"), 0);
        assert_eq!(guard.stats().total_inspected, 0);
    }

    #[test]
    fn test_guard_window_expiration() {
        let mut guard = SctGuard::new(3).expect("valid guard").with_window_size(10);
        let bad = StuartianTensor::new(0.9, 0.1, -0.5).unwrap();

        guard.inspect_payload("node".into(), bad).unwrap();
        assert_eq!(guard.get_violation_count("node"), 1);

        // Advance past window
        for _ in 0..11 {
            guard.advance_tick();
        }

        assert_eq!(guard.get_violation_count("node"), 0);
    }

    #[test]
    fn test_guard_default() {
        let guard = SctGuard::default();
        assert_eq!(guard.max_violations, 3);
    }

    #[test]
    fn test_stats_rejection_rate_empty() {
        let stats = SctGuardStats::default();
        assert_eq!(stats.rejection_rate(), 0.0);
    }

    #[test]
    fn test_error_display() {
        let err = SctGuardError::MaxViolationsExceeded {
            node_id: "n1".into(),
            count: 5,
            max: 3,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("n1"));
    }
}
