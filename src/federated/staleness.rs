//! Staleness-Aware Weighting — Ponderación por obsolescencia asíncrona.
//!
//! Fórmula estricta: `w = 1.0 / (1.0 + tau).powf(alpha)`
//! donde `tau = max(global_version - local_version, 0)`.
//!
//! Ley 2 (Reconocimiento del Error): decay por staleness + rejection de versiones futuras.
//! Ley 5 (Múltiples posibilidades): tolerancia asíncrona, convergencia eventual.
//!
//! Feature gate: `#[cfg(feature = "v2.1-staleness-aware")]`

use std::fmt;

// ─── Errors ───

#[derive(Debug, Clone, PartialEq)]
pub enum StalenessError {
    FutureVersion { local: u64, global: u64 },
    InvalidAlpha(f32),
    WeightOutOfBounds(f32),
}

impl fmt::Display for StalenessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StalenessError::FutureVersion { local, global } => {
                write!(
                    f,
                    "Versión futura detectada: local={}, global={}",
                    local, global
                )
            }
            StalenessError::InvalidAlpha(alpha) => {
                write!(f, "Alpha inválido: {} (debe ser > 0)", alpha)
            }
            StalenessError::WeightOutOfBounds(w) => {
                write!(f, "Peso fuera de rango: {} (debe estar en [0.0, 1.0])", w)
            }
        }
    }
}

impl std::error::Error for StalenessError {}

// ─── Core Decay Function ───

/// Calcula peso de decaimiento por obsolescencia.
///
/// Fórmula: `w = 1.0 / (1.0 + tau).powf(alpha)`
/// donde `tau = max(global_version - local_version, 0)`.
///
/// # Arguments
/// * `local_version` - Versión del gradiente local.
/// * `global_version` - Versión global actual.
/// * `alpha` - Exponente de decaimiento (debe ser > 0).
///
/// # Returns
/// Peso en rango `[0.0, 1.0]`. Retorna `Err(FutureVersion)` si `local > global`.
///
/// # Panics
/// No panics. Valida límites de alpha y peso resultante.
pub fn apply_staleness_decay(
    local_version: u64,
    global_version: u64,
    alpha: f32,
) -> Result<f32, StalenessError> {
    if alpha <= 0.0 {
        return Err(StalenessError::InvalidAlpha(alpha));
    }

    if local_version > global_version {
        return Err(StalenessError::FutureVersion {
            local: local_version,
            global: global_version,
        });
    }

    let tau = global_version.saturating_sub(local_version) as f32;
    let w = 1.0 / (1.0 + tau).powf(alpha);

    // Validar límites
    if !(0.0..=1.0).contains(&w) {
        return Err(StalenessError::WeightOutOfBounds(w));
    }

    Ok(w)
}

// ─── StalenessConfig ───

/// Configuración de decaimiento por obsolescencia.
#[derive(Debug, Clone, PartialEq)]
pub struct StalenessConfig {
    /// Exponente de decaimiento (alpha > 0).
    /// Mayor alpha = decaimiento más agresivo.
    pub alpha: f32,
    /// Umbral mínimo de peso antes de rechazo.
    pub min_weight: f32,
    /// Versión global actual.
    pub global_version: u64,
}

impl StalenessConfig {
    pub fn new(alpha: f32, min_weight: f32, global_version: u64) -> Result<Self, StalenessError> {
        if alpha <= 0.0 {
            return Err(StalenessError::InvalidAlpha(alpha));
        }
        if !(0.0..=1.0).contains(&min_weight) {
            return Err(StalenessError::WeightOutOfBounds(min_weight));
        }
        Ok(Self {
            alpha,
            min_weight,
            global_version,
        })
    }

    /// Aplicar decaimiento y verificar umbral mínimo.
    pub fn evaluate(&self, local_version: u64) -> Result<f32, StalenessError> {
        let w = apply_staleness_decay(local_version, self.global_version, self.alpha)?;
        if w < self.min_weight {
            return Err(StalenessError::WeightOutOfBounds(w));
        }
        Ok(w)
    }

    /// Actualizar versión global.
    pub fn advance_global_version(&mut self, new_version: u64) {
        self.global_version = new_version;
    }
}

// ─── Tensor Weighting (candle-core integration) ───

/// Aplica peso de staleness a gradiente como slice de f32.
pub fn weight_gradients(gradients: &[f32], weight: f32) -> Vec<f32> {
    gradients.iter().map(|g| g * weight).collect()
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-6;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    fn test_decay_tau_zero() {
        // tau = 0 → w = 1.0
        let w = apply_staleness_decay(5, 5, 0.5).unwrap();
        assert!(approx_eq(w, 1.0), "Expected w=1.0 for tau=0, got {}", w);
    }

    #[test]
    fn test_decay_tau_one() {
        // tau = 1, alpha = 0.5 → w = 1/(1+1)^0.5 = 1/sqrt(2) ≈ 0.7071068
        let w = apply_staleness_decay(4, 5, 0.5).unwrap();
        let expected = 1.0 / 2.0f32.sqrt();
        assert!(
            approx_eq(w, expected),
            "Expected w={}, got {}",
            expected,
            w
        );
    }

    #[test]
    fn test_decay_tau_five() {
        // tau = 5, alpha = 1.0 → w = 1/(1+5)^1 = 1/6 ≈ 0.1666667
        let w = apply_staleness_decay(10, 15, 1.0).unwrap();
        let expected = 1.0 / 6.0;
        assert!(
            approx_eq(w, expected),
            "Expected w={}, got {}",
            expected,
            w
        );
    }

    #[test]
    fn test_decay_tau_ten() {
        // tau = 10, alpha = 2.0 → w = 1/(1+10)^2 = 1/121 ≈ 0.0082645
        let w = apply_staleness_decay(20, 30, 2.0).unwrap();
        let expected = 1.0 / 121.0;
        assert!(
            approx_eq(w, expected),
            "Expected w={}, got {}",
            expected,
            w
        );
    }

    #[test]
    fn test_decay_future_version() {
        let result = apply_staleness_decay(100, 50, 0.5);
        match result {
            Err(StalenessError::FutureVersion { local, global }) => {
                assert_eq!(local, 100);
                assert_eq!(global, 50);
            }
            other => panic!("Expected FutureVersion, got {:?}", other),
        }
    }

    #[test]
    fn test_decay_invalid_alpha_zero() {
        let result = apply_staleness_decay(5, 10, 0.0);
        assert_eq!(result, Err(StalenessError::InvalidAlpha(0.0)));
    }

    #[test]
    fn test_decay_invalid_alpha_negative() {
        let result = apply_staleness_decay(5, 10, -1.0);
        assert_eq!(result, Err(StalenessError::InvalidAlpha(-1.0)));
    }

    #[test]
    fn test_decay_monotonic_decrease() {
        // Mayor tau → menor peso
        let w1 = apply_staleness_decay(9, 10, 1.0).unwrap(); // tau=1
        let w2 = apply_staleness_decay(5, 10, 1.0).unwrap(); // tau=5
        let w3 = apply_staleness_decay(0, 10, 1.0).unwrap(); // tau=10
        assert!(w1 > w2, "w(tau=1) > w(tau=5): {} > {}", w1, w2);
        assert!(w2 > w3, "w(tau=5) > w(tau=10): {} > {}", w2, w3);
    }

    #[test]
    fn test_decay_alpha_sensitivity() {
        // Mayor alpha → decaimiento más agresivo
        let w_low = apply_staleness_decay(5, 10, 0.1).unwrap();
        let w_high = apply_staleness_decay(5, 10, 2.0).unwrap();
        assert!(
            w_low > w_high,
            "alpha=0.1 > alpha=2.0: {} > {}",
            w_low,
            w_high
        );
    }

    #[test]
    fn test_weight_gradients() {
        let grads = vec![1.0, 2.0, 3.0, 4.0];
        let weighted = weight_gradients(&grads, 0.5);
        assert_eq!(weighted, vec![0.5, 1.0, 1.5, 2.0]);
    }

    #[test]
    fn test_weight_gradients_zero() {
        let grads = vec![1.0, 2.0, 3.0];
        let weighted = weight_gradients(&grads, 0.0);
        assert_eq!(weighted, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_weight_gradients_full() {
        let grads = vec![1.0, 2.0, 3.0];
        let weighted = weight_gradients(&grads, 1.0);
        assert_eq!(weighted, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_config_new() {
        let config = StalenessConfig::new(1.0, 0.01, 100).unwrap();
        assert_eq!(config.alpha, 1.0);
        assert_eq!(config.min_weight, 0.01);
        assert_eq!(config.global_version, 100);
    }

    #[test]
    fn test_config_invalid_alpha() {
        let result = StalenessConfig::new(0.0, 0.01, 100);
        assert_eq!(result, Err(StalenessError::InvalidAlpha(0.0)));
    }

    #[test]
    fn test_config_evaluate_above_threshold() {
        let config = StalenessConfig::new(1.0, 0.01, 10).unwrap();
        let w = config.evaluate(9).unwrap(); // tau=1 → w=0.5
        assert!(approx_eq(w, 0.5));
    }

    #[test]
    fn test_config_evaluate_below_threshold() {
        let config = StalenessConfig::new(1.0, 0.5, 100).unwrap();
        let result = config.evaluate(0); // tau=100 → w=0.0099
        match result {
            Err(StalenessError::WeightOutOfBounds(w)) => {
                assert!(w < 0.5, "Weight {} below threshold 0.5", w);
            }
            other => panic!("Expected WeightOutOfBounds, got {:?}", other),
        }
    }

    #[test]
    fn test_config_advance_version() {
        let mut config = StalenessConfig::new(1.0, 0.01, 10).unwrap();
        config.advance_global_version(20);
        assert_eq!(config.global_version, 20);
    }

    #[test]
    fn test_error_display() {
        let err = StalenessError::FutureVersion { local: 10, global: 5 };
        let msg = format!("{}", err);
        assert!(msg.contains("futura"));
    }
}
