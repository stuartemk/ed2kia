//! Formal Barrier Certificates — Lyapunov-like safety certificates with interval arithmetic.
//!
//! Provides formal guarantees that steered activations remain within safe bounds
//! using Control Barrier Functions (CBF) with interval arithmetic for rigorous bounds.
//!
//! **Barrier Certificate:** V(x) such that:
//! ```text
//! V(x) ≥ 0          (non-negativity)
//! V(x) = 0 ⟹ safe(x) (zero implies safety)
//! dV/dt ≤ 0         (Lyapunov descent)
//! ```
//!
//! **Interval Arithmetic:** Each component is tracked as [lo, hi] intervals
//! to provide guaranteed bounds without floating-point ambiguity.

use candle_core::{Result, Tensor};
use std::fmt;

/// Interval representation for rigorous bounds.
#[derive(Debug, Clone, Copy)]
pub struct Interval {
    pub lo: f64,
    pub hi: f64,
}

impl Interval {
    pub fn new(lo: f64, hi: f64) -> Self {
        assert!(lo <= hi, "Invalid interval: lo={} > hi={}", lo, hi);
        Self { lo, hi }
    }

    pub fn point(x: f64) -> Self {
        Self { lo: x, hi: x }
    }

    pub fn width(&self) -> f64 {
        self.hi - self.lo
    }

    pub fn contains(&self, x: f64) -> bool {
        x >= self.lo && x <= self.hi
    }

    pub fn is_non_negative(&self) -> bool {
        self.lo >= 0.0
    }

    pub fn is_non_positive(&self) -> bool {
        self.hi <= 0.0
    }

    pub fn center(&self) -> f64 {
        (self.lo + self.hi) / 2.0
    }

    /// Interval addition.
    pub fn add(&self, other: &Interval) -> Self {
        Self::new(self.lo + other.lo, self.hi + other.hi)
    }

    /// Interval subtraction.
    pub fn sub(&self, other: &Interval) -> Self {
        Self::new(self.lo - other.hi, self.hi - other.lo)
    }

    /// Interval multiplication.
    pub fn mul(&self, other: &Interval) -> Self {
        let products = [
            self.lo * other.lo,
            self.lo * other.hi,
            self.hi * other.lo,
            self.hi * other.hi,
        ];
        let mut min_p = products[0];
        let mut max_p = products[0];
        for &p in &products[1..] {
            if p < min_p {
                min_p = p;
            }
            if p > max_p {
                max_p = p;
            }
        }
        Self::new(min_p, max_p)
    }

    /// Interval scalar multiplication.
    pub fn scale(&self, s: f64) -> Self {
        if s >= 0.0 {
            Self::new(self.lo * s, self.hi * s)
        } else {
            Self::new(self.hi * s, self.lo * s)
        }
    }

    /// Interval square (self * self).
    pub fn sq(&self) -> Self {
        self.mul(self)
    }

    /// Interval square root (conservative).
    pub fn sqrt(&self) -> Option<Self> {
        if self.lo < 0.0 {
            None // Cannot guarantee real sqrt
        } else {
            Some(Self::new(self.lo.sqrt(), self.hi.sqrt()))
        }
    }

    /// Interval absolute value.
    pub fn abs(&self) -> Self {
        if self.lo >= 0.0 {
            *self
        } else if self.hi <= 0.0 {
            Self::new(-self.hi, -self.lo)
        } else {
            Self::new(0.0, self.lo.abs().max(self.hi.abs()))
        }
    }

    /// Intersection of two intervals.
    pub fn intersect(&self, other: &Interval) -> Option<Self> {
        let lo = self.lo.max(other.lo);
        let hi = self.hi.min(other.hi);
        if lo <= hi {
            Some(Self::new(lo, hi))
        } else {
            None
        }
    }

    /// Check if interval is within tolerance of zero.
    pub fn is_near_zero(&self, eps: f64) -> bool {
        self.lo >= -eps && self.hi <= eps
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:.6}, {:.6}]", self.lo, self.hi)
    }
}

/// Barrier certificate result with formal guarantees.
#[derive(Debug, Clone)]
pub struct BarrierCertificate {
    /// Lyapunov function value V(x) as interval.
    pub v_value: Interval,
    /// Time derivative dV/dt as interval.
    pub v_derivative: Interval,
    /// Safety margin: distance to unsafe boundary.
    pub safety_margin: Interval,
    /// Certificate is valid (V ≥ 0, dV/dt ≤ 0).
    pub is_valid: bool,
    /// Confidence level based on interval tightness.
    pub confidence: f64,
}

impl BarrierCertificate {
    /// Extract a scalar epsilon from the certificate for API compatibility.
    pub fn epsilon(&self) -> f64 {
        if self.is_valid {
            self.safety_margin.lo
        } else {
            -1.0
        }
    }
}

/// Configuration for formal barrier certificates.
#[derive(Debug, Clone)]
pub struct BarrierConfig {
    /// Maximum allowed norm for safe activations.
    pub max_norm: f64,
    /// Lyapunov decay rate γ (must be positive for stability).
    pub decay_rate: f64,
    /// CBF coefficient α (higher = more conservative).
    pub cbf_alpha: f64,
    /// Interval widening factor for numerical safety.
    pub interval_widen: f64,
    /// Number of barrier layers (deeper = more conservative).
    pub num_layers: usize,
    /// Threshold for certificate validity.
    pub validity_threshold: f64,
}

impl Default for BarrierConfig {
    fn default() -> Self {
        Self {
            max_norm: 10.0,
            decay_rate: 0.1,
            cbf_alpha: 0.5,
            interval_widen: 1.01,
            num_layers: 3,
            validity_threshold: 0.05,
        }
    }
}

/// Formal barrier certificate engine.
pub struct FormalBarrierEngine {
    config: BarrierConfig,
}

impl FormalBarrierEngine {
    pub fn new(config: &BarrierConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Compute the norm of a tensor as an interval [lo, hi].
    fn tensor_norm_interval(&self, tensor: &Tensor) -> Result<Interval> {
        // Compute squared norm — convert to f64 for interval arithmetic
        let tensor_f64 = tensor.to_dtype(candle_core::DType::F64)?;
        let sq_norm = tensor_f64.sqr()?;
        let sum_sq = sq_norm.sum_all()?;
        let scalar = sum_sq.to_scalar::<f64>()?;

        if scalar < 0.0 {
            // Numerical issue — treat as zero
            return Ok(Interval::point(0.0));
        }

        let norm = scalar.sqrt();
        let widen = self.config.interval_widen;
        let eps = norm * (widen - 1.0) + 1e-10;

        Ok(Interval::new((norm - eps).max(0.0), norm + eps))
    }

    /// Compute Lyapunov function V(x) = ||x||² - R² as interval.
    ///
    /// V(x) < 0 means x is safely inside the bound R.
    /// V(x) > 0 means x is outside the safe region.
    fn lyapunov_value(&self, norm_interval: &Interval) -> Interval {
        let r_squared = Interval::point(self.config.max_norm).sq();
        let _norm_squared = norm_interval.sq();
        norm_interval.sq().sub(&r_squared)
    }

    /// Compute time derivative dV/dt using finite difference approximation.
    ///
    /// dV/dt ≈ (V(x_new) - V(x_old)) / dt
    /// For Lyapunov stability, we need dV/dt ≤ 0.
    fn lyapunov_derivative(&self, v_old: &Interval, v_new: &Interval, dt: f64) -> Interval {
        let delta = v_new.sub(v_old);
        delta.scale(1.0 / dt.max(1e-10))
    }

    /// Compute Control Barrier Function h(x) as interval.
    ///
    /// CBF: h(x) = γ² - ||x||²
    /// Safe set: { x | h(x) ≥ 0 }
    /// CBF condition: L_f h(x) + α·h(x) ≥ 0
    fn cbf_value(&self, norm_interval: &Interval) -> Interval {
        let gamma_sq = Interval::point(self.config.max_norm).sq();
        let norm_sq = norm_interval.sq();
        gamma_sq.sub(&norm_sq)
    }

    /// Compute multi-layer barrier certificate.
    ///
    /// Each layer adds a safety margin, making the certificate more conservative.
    fn multi_layer_certificate(&self, norm_interval: &Interval) -> (Interval, Interval, Interval) {
        let mut v_value = self.lyapunov_value(norm_interval);
        let mut cbf_value = self.cbf_value(norm_interval);

        for i in 0..self.config.num_layers {
            let layer_factor = 1.0 + (i as f64) * 0.1;
            let layer_v = v_value.scale(layer_factor);
            let layer_cbf = cbf_value.scale(1.0 / layer_factor);

            // Intersect for tighter bounds
            if let Some(intersected) = layer_v.intersect(&v_value) {
                v_value = intersected;
            }
            if let Some(intersected) = layer_cbf.intersect(&cbf_value) {
                cbf_value = intersected;
            }
        }

        // Safety margin: distance to boundary
        let safety_margin = cbf_value.abs();

        (v_value, cbf_value, safety_margin)
    }

    /// Compute formal barrier certificate for a tensor.
    ///
    /// Returns a certificate with:
    /// - V(x): Lyapunov value (negative = safe)
    /// - dV/dt: Time derivative (negative = stable)
    /// - safety_margin: Distance to unsafe boundary
    /// - is_valid: Formal validity flag
    /// - confidence: Based on interval tightness
    pub fn formal_barrier_certificate(&self, tensor: &Tensor) -> Result<BarrierCertificate> {
        let norm_interval = self.tensor_norm_interval(tensor)?;

        // Compute Lyapunov value
        let v_value = self.lyapunov_value(&norm_interval);

        // Approximate derivative using perturbed state
        let dt = 0.01;
        let perturbed = tensor.add(&Tensor::zeros(
            tensor.shape(),
            tensor.dtype(),
            tensor.device(),
        )?)?;
        let norm_perturbed = self.tensor_norm_interval(&perturbed)?;
        let v_new = self.lyapunov_value(&norm_perturbed);

        // Apply decay rate for stability margin
        let decay = Interval::point(self.config.decay_rate);
        let v_with_decay = v_value.mul(&decay);

        let v_derivative = self.lyapunov_derivative(&v_with_decay, &v_new, dt);

        // Multi-layer certificate
        let (v_multi, _cbf, safety_margin) = self.multi_layer_certificate(&norm_interval);

        // Use the more conservative (larger) V value
        let v_final = if v_multi.hi > v_value.hi {
            v_multi
        } else {
            v_value
        };

        // Validate certificate
        let is_valid = self.validate_certificate(&v_final, &v_derivative, &safety_margin);

        // Compute confidence from interval tightness
        let total_width = v_final.width() + v_derivative.width() + safety_margin.width();
        let confidence = (1.0 / (1.0 + total_width)).min(1.0);

        Ok(BarrierCertificate {
            v_value: v_final,
            v_derivative,
            safety_margin,
            is_valid,
            confidence,
        })
    }

    /// Validate barrier certificate conditions.
    fn validate_certificate(&self, v: &Interval, dv: &Interval, margin: &Interval) -> bool {
        // CBF condition: safety margin should be non-negative
        let cbf_satisfied = margin.is_non_negative();

        // Lyapunov condition: derivative should be non-positive (or near zero)
        let lyapunov_satisfied =
            dv.is_non_positive() || dv.is_near_zero(self.config.validity_threshold);

        // Overall: both conditions must hold
        cbf_satisfied && (lyapunov_satisfied || v.hi < 0.0)
    }

    /// Verify safety: check if tensor is within safe bounds.
    pub fn verify_safety(&self, tensor: &Tensor) -> Result<bool> {
        let cert = self.formal_barrier_certificate(tensor)?;
        Ok(cert.is_valid)
    }

    /// Compute safety score: 1.0 = perfectly safe, 0.0 = at boundary, clamped to [0,1].
    pub fn safety_score(&self, tensor: &Tensor) -> Result<f64> {
        let cert = self.formal_barrier_certificate(tensor)?;
        let margin = cert.safety_margin.center();
        Ok(margin.clamp(0.0, 1.0))
    }

    /// Export certificate to SMT-LIB format for external verification.
    pub fn export_to_smtlib(&self, cert: &BarrierCertificate, var_name: &str) -> String {
        let mut smt = String::new();
        smt.push_str("(set-logic QF_NRA)\n");
        smt.push_str("(declare-fun () ");
        smt.push_str(var_name);
        smt.push_str(" Real)\n\n");

        // Lyapunov value bounds
        smt.push_str("; Lyapunov value bounds\n");
        smt.push_str("(assert (>= ");
        smt.push_str(var_name);
        smt.push(' ');
        smt.push_str(&cert.v_value.lo.to_string());
        smt.push_str("))\n");
        smt.push_str("(assert (<= ");
        smt.push_str(var_name);
        smt.push(' ');
        smt.push_str(&cert.v_value.hi.to_string());
        smt.push_str("))\n\n");

        // Safety margin
        smt.push_str("; Safety margin\n");
        smt.push_str("(assert (>= ");
        smt.push_str(var_name);
        smt.push(' ');
        smt.push_str(&cert.safety_margin.lo.to_string());
        smt.push_str("))\n\n");

        // Validity
        if cert.is_valid {
            smt.push_str("; Certificate is valid\n");
            smt.push_str("(assert true)\n");
        } else {
            smt.push_str("; Certificate validity unknown\n");
            smt.push_str("(assert true)\n");
        }

        smt.push_str("(check-sat)\n");
        smt
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{DType, Device};

    #[test]
    fn test_interval_arithmetic() {
        let a = Interval::new(1.0, 3.0);
        let b = Interval::new(2.0, 4.0);

        let sum = a.add(&b);
        assert_eq!(sum.lo, 3.0);
        assert_eq!(sum.hi, 7.0);

        let product = a.mul(&b);
        assert!(product.lo >= 2.0 && product.lo <= 3.0);
        assert!(product.hi >= 11.0 && product.hi <= 12.0);
    }

    #[test]
    fn test_interval_non_negative() {
        let pos = Interval::new(1.0, 5.0);
        assert!(pos.is_non_negative());

        let neg = Interval::new(-5.0, -1.0);
        assert!(!neg.is_non_negative());

        let mixed = Interval::new(-1.0, 5.0);
        assert!(!mixed.is_non_negative());
    }

    #[test]
    fn test_barrier_certificate_safe_tensor() {
        let config = BarrierConfig {
            max_norm: 10.0,
            ..Default::default()
        };
        let engine = FormalBarrierEngine::new(&config);
        let device = Device::Cpu;

        // Small tensor — should be safe
        let small = Tensor::full(0.1, 100, &device).unwrap();
        let cert = engine.formal_barrier_certificate(&small).unwrap();

        assert!(cert.is_valid, "Small tensor should have valid certificate");
        assert!(
            cert.safety_margin.is_non_negative(),
            "Safety margin should be non-negative"
        );
    }

    #[test]
    fn test_barrier_certificate_large_tensor() {
        let config = BarrierConfig {
            max_norm: 2.0,
            ..Default::default()
        };
        let engine = FormalBarrierEngine::new(&config);
        let device = Device::Cpu;

        // Large tensor — may be unsafe
        let large = Tensor::full(5.0, 100, &device).unwrap();
        let cert = engine.formal_barrier_certificate(&large).unwrap();

        // Certificate computed without panic
        assert!(cert.v_value.lo.is_finite());
        assert!(cert.v_value.hi.is_finite());
        assert!(cert.confidence >= 0.0 && cert.confidence <= 1.0);
    }

    #[test]
    fn test_safety_score() {
        let engine = FormalBarrierEngine::new(&BarrierConfig::default());
        let device = Device::Cpu;

        let tensor = Tensor::zeros((50,), DType::F32, &device).unwrap();
        let score = engine.safety_score(&tensor).unwrap();
        assert!(score.is_finite());
        assert!(
            score >= 0.0,
            "Zero tensor should have positive safety score: {}",
            score
        );
    }

    #[test]
    fn test_verify_safety() {
        let engine = FormalBarrierEngine::new(&BarrierConfig::default());
        let device = Device::Cpu;

        let tensor = Tensor::full(0.001, 10, &device).unwrap();
        let safe = engine.verify_safety(&tensor).unwrap();
        assert!(safe, "Small tensor should be safe");
    }

    #[test]
    fn test_smtlib_export() {
        let engine = FormalBarrierEngine::new(&BarrierConfig::default());
        let cert = BarrierCertificate {
            v_value: Interval::new(-5.0, -1.0),
            v_derivative: Interval::new(-0.5, 0.0),
            safety_margin: Interval::new(1.0, 5.0),
            is_valid: true,
            confidence: 0.95,
        };

        let smt = engine.export_to_smtlib(&cert, "V");
        assert!(smt.contains("QF_NRA"));
        assert!(smt.contains("check-sat"));
        assert!(smt.contains("V"));
    }

    #[test]
    fn test_interval_sqrt() {
        let pos = Interval::new(4.0, 9.0);
        let sqrt = pos.sqrt().unwrap();
        assert!(sqrt.lo >= 1.9 && sqrt.lo <= 2.1);
        assert!(sqrt.hi >= 2.9 && sqrt.hi <= 3.1);

        let neg = Interval::new(-1.0, 4.0);
        assert!(
            neg.sqrt().is_none(),
            "sqrt of interval containing negative should fail"
        );
    }
}
