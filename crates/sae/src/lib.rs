//! ed2k-sae — Sparse Autoencoder (SAE) Module
//!
//! Distributed interpretability through Sparse Autoencoders.
//! Handles model loading, feature extraction, and SAE routing.
//!
//! **Sprint 135 — Matryoshka SAEs & Drift-Plus-Penalty Scheduler:**
//! Implements dynamic sparsity via Matryoshka doll sub-matrices, allowing
//! nodes to evaluate only the inner dimensions based on available energy.
//! The Lyapunov Drift-Plus-Penalty scheduler optimizes:
//! ```math
//! utility = E[f_i] - V · (energy_cost + queue_delay)
//! ```
//! Resolution: 1.0 (full) / 0.25 (core) / 0.0 (delegate).

pub mod loader;
pub mod manifold;
pub mod router;
pub mod scheduler;

use candle_core::{DType, Device, Result, Tensor};

/// Matryoshka SAE — Dynamic Resolution Sparse Autoencoder.
///
/// Supports nested representations (Matryoshka dolls) where the first `k`
/// dimensions form a valid embedding at any resolution level. This enables
/// energy-aware evaluation: low-battery nodes use only the core features,
/// while high-energy nodes evaluate the full representation.
///
/// **Loss:** `L = ||x - ψ(φ(x))||² + λ·Σlog(1+|a_i|) + top-k_sparsity`
#[derive(Debug)]
pub struct MatryoshkaSAE {
    /// Encoder weights: shape `[d_model, d_hidden]`
    pub encoder_weights: Tensor,
    /// Decoder weights: shape `[d_hidden, d_model]`
    pub decoder_weights: Tensor,
    /// Sparsity regularization coefficient λ
    pub sparsity_lambda: f32,
}

impl MatryoshkaSAE {
    /// Create a new Matryoshka SAE.
    pub fn new(encoder_weights: Tensor, decoder_weights: Tensor) -> Self {
        Self {
            encoder_weights,
            decoder_weights,
            sparsity_lambda: 0.01,
        }
    }

    /// Create with custom sparsity λ.
    pub fn with_sparsity_lambda(mut self, lambda: f32) -> Self {
        self.sparsity_lambda = lambda;
        self
    }

    /// Lyapunov Drift-Plus-Penalty Scheduler.
    ///
    /// Returns the Matryoshka resolution level based on thermodynamic utility:
    /// ```math
    /// utility = E[f_i] - V · (energy_cost + queue_delay)
    /// ```
    ///
    /// - `utility < 0` → 0.0 (delegate to network)
    /// - `0 ≤ utility < 5` → 0.25 (core-only, low energy)
    /// - `utility ≥ 5` → 1.0 (full evaluation)
    ///
    /// # Arguments
    /// * `expected_fitness` — Expected symbiotic benefit `E[f_i]`
    /// * `energy_cost` — Estimated battery cost
    /// * `queue_delay` — Network/processing latency
    /// * `v_param` — Trade-off parameter V (higher = more conservative)
    #[allow(dead_code)]
    pub fn compute_drift_plus_penalty(
        expected_fitness: f32,
        energy_cost: f32,
        queue_delay: f32,
        v_param: f32,
    ) -> f32 {
        let utility = expected_fitness - v_param * (energy_cost + queue_delay);

        if utility < 0.0 {
            0.0 // Delegate — thermodynamic cost exceeds benefit
        } else if utility < 5.0 {
            0.25 // Low-energy mode: Matryoshka Core (25% features)
        } else {
            1.0 // Full evaluation
        }
    }

    /// Matryoshka SAE Forward Pass (Dynamic Sparsity).
    ///
    /// Selects only the first `active_dim = total_dim * resolution_fraction`
    /// rows of the encoder and corresponding columns of the decoder,
    /// performing a reduced forward pass.
    ///
    /// **Loss:** `L = ||x - ψ(φ(x))||² + λ·Σlog(1+|a_i|) + top-k`
    ///
    /// # Arguments
    /// * `hidden_state` — Input tensor `[batch, d_model]`
    /// * `resolution_fraction` — Resolution level (0.25, 0.5, 1.0)
    ///
    /// # Returns
    /// Reconstructed tensor at the selected resolution.
    #[allow(dead_code)]
    pub fn forward_matryoshka(
        &self,
        hidden_state: &Tensor,
        resolution_fraction: f32,
    ) -> Result<Tensor> {
        let total_dim = self.decoder_weights.dim(0)?;
        let active_dim = ((total_dim as f32) * resolution_fraction).max(1.0) as usize;

        // Extract sub-matrix (Matryoshka doll interior)
        let w_dec_sub = self.decoder_weights.narrow(0, 0, active_dim)?;
        let w_enc_sub = self.encoder_weights.narrow(1, 0, active_dim)?;

        // Forward pass on reduced dimension
        let acts = hidden_state.matmul(&w_enc_sub)?.relu()?;
        let reconstructed = acts.matmul(&w_dec_sub.t()?)?;

        Ok(reconstructed)
    }

    /// Compute sparsity penalty: `λ · Σ log(1 + |a_i|)`
    #[allow(dead_code)]
    pub fn sparsity_penalty(&self, activations: &Tensor) -> Result<Tensor> {
        let abs_acts = activations.abs()?;
        let log_term = (abs_acts + 1f32).log()?;
        let sum = log_term.sum_all()?.to_scalar::<f32>()?;
        let lambda = Tensor::new(&[self.sparsity_lambda], activations.device())?;
        Ok(lambda * sum)
    }
}

/// Resolution levels for Matryoshka SAE.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MatryoshkaResolution {
    /// Delegate — skip evaluation entirely.
    Delegate = 0,
    /// Core only — 25% of features.
    Core = 1,
    /// Medium — 50% of features.
    Medium = 2,
    /// Full — 100% of features.
    Full = 3,
}

impl MatryoshkaResolution {
    /// Convert to fraction for `forward_matryoshka`.
    pub fn fraction(&self) -> f32 {
        match self {
            MatryoshkaResolution::Delegate => 0.0,
            MatryoshkaResolution::Core => 0.25,
            MatryoshkaResolution::Medium => 0.5,
            MatryoshkaResolution::Full => 1.0,
        }
    }
}

impl From<f32> for MatryoshkaResolution {
    fn from(value: f32) -> Self {
        if value < 0.01 {
            MatryoshkaResolution::Delegate
        } else if value < 0.5 {
            MatryoshkaResolution::Core
        } else if value < 0.75 {
            MatryoshkaResolution::Medium
        } else {
            MatryoshkaResolution::Full
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drift_plus_penalty_delegates() {
        // High energy cost → delegate
        let res = MatryoshkaSAE::compute_drift_plus_penalty(2.0, 4.0, 3.0, 1.0);
        assert_eq!(res, 0.0);
    }

    #[test]
    fn test_drift_plus_penalty_low_energy() {
        // Moderate utility → core mode
        let res = MatryoshkaSAE::compute_drift_plus_penalty(12.0, 4.0, 1.5, 2.0);
        assert_eq!(res, 0.25);
    }

    #[test]
    fn test_drift_plus_penalty_full() {
        // High utility → full evaluation
        let res = MatryoshkaSAE::compute_drift_plus_penalty(20.0, 1.0, 0.5, 1.0);
        assert_eq!(res, 1.0);
    }

    #[test]
    fn test_drift_plus_penalty_zero_cost() {
        // Zero cost → full
        let res = MatryoshkaSAE::compute_drift_plus_penalty(10.0, 0.0, 0.0, 1.0);
        assert_eq!(res, 1.0);
    }

    #[test]
    fn test_drift_plus_penalty_boundary_positive() {
        // Exactly at boundary utility = 5
        let res = MatryoshkaSAE::compute_drift_plus_penalty(10.0, 2.0, 0.5, 2.0);
        // utility = 10 - 2*(2+0.5) = 10 - 5 = 5 → full
        assert_eq!(res, 1.0);
    }

    #[test]
    fn test_drift_plus_penalty_boundary_negative() {
        // Just below zero
        let res = MatryoshkaSAE::compute_drift_plus_penalty(3.0, 2.0, 1.1, 1.0);
        // utility = 3 - 1*(2+1.1) = -0.1 → delegate
        assert_eq!(res, 0.0);
    }

    #[test]
    fn test_drift_plus_penalty_high_v_param() {
        // High V → conservative → delegate
        let res = MatryoshkaSAE::compute_drift_plus_penalty(10.0, 2.0, 1.0, 10.0);
        // utility = 10 - 10*(2+1) = -20 → delegate
        assert_eq!(res, 0.0);
    }

    #[test]
    fn test_drift_plus_penalty_low_v_param() {
        // Low V → aggressive → full
        let res = MatryoshkaSAE::compute_drift_plus_penalty(10.0, 2.0, 1.0, 0.1);
        // utility = 10 - 0.1*(2+1) = 9.7 → full
        assert_eq!(res, 1.0);
    }

    #[test]
    fn test_matryoshka_resolution_fraction() {
        assert_eq!(MatryoshkaResolution::Delegate.fraction(), 0.0);
        assert_eq!(MatryoshkaResolution::Core.fraction(), 0.25);
        assert_eq!(MatryoshkaResolution::Medium.fraction(), 0.5);
        assert_eq!(MatryoshkaResolution::Full.fraction(), 1.0);
    }

    #[test]
    fn test_matryoshka_resolution_from_f32() {
        assert_eq!(MatryoshkaResolution::from(0.0), MatryoshkaResolution::Delegate);
        assert_eq!(MatryoshkaResolution::from(0.25), MatryoshkaResolution::Core);
        assert_eq!(MatryoshkaResolution::from(0.5), MatryoshkaResolution::Medium);
        assert_eq!(MatryoshkaResolution::from(1.0), MatryoshkaResolution::Full);
    }

    #[test]
    fn test_matryoshka_resolution_roundtrip() {
        for res in [
            MatryoshkaResolution::Delegate,
            MatryoshkaResolution::Core,
            MatryoshkaResolution::Medium,
            MatryoshkaResolution::Full,
        ] {
            let frac = res.fraction();
            let back: MatryoshkaResolution = frac.into();
            // Delegate maps to 0.0 which rounds back to Delegate
            assert_eq!(res, back, "Roundtrip failed for {:?}", res);
        }
    }

    #[test]
    fn test_matryoshka_sae_new() {
        let device = Device::Cpu;
        let enc = Tensor::randn(0f32, 1f32, [128, 256], &device).unwrap();
        let dec = Tensor::randn(0f32, 1f32, [256, 128], &device).unwrap();
        let sae = MatryoshkaSAE::new(enc, dec);
        assert_eq!(sae.sparsity_lambda, 0.01);
    }

    #[test]
    fn test_matryoshka_sae_with_sparsity_lambda() {
        let device = Device::Cpu;
        let enc = Tensor::randn(0f32, 1f32, [128, 256], &device).unwrap();
        let dec = Tensor::randn(0f32, 1f32, [256, 128], &device).unwrap();
        let sae = MatryoshkaSAE::new(enc, dec).with_sparsity_lambda(0.1);
        assert_eq!(sae.sparsity_lambda, 0.1);
    }

    #[test]
    fn test_forward_matryoshka_full_resolution() {
        let device = Device::Cpu;
        let enc = Tensor::randn(0f32, 1f32, [4, 8], &device).unwrap();
        let dec = Tensor::randn(0f32, 1f32, [8, 4], &device).unwrap();
        let sae = MatryoshkaSAE::new(enc, dec);
        let input = Tensor::ones(1f32, [1, 4], &device).unwrap();
        let out = sae.forward_matryoshka(&input, 1.0).unwrap();
        assert_eq!(out.shape().dims(), &[1, 4]);
    }

    #[test]
    fn test_forward_matryoshka_core_resolution() {
        let device = Device::Cpu;
        let enc = Tensor::randn(0f32, 1f32, [4, 8], &device).unwrap();
        let dec = Tensor::randn(0f32, 1f32, [8, 4], &device).unwrap();
        let sae = MatryoshkaSAE::new(enc, dec);
        let input = Tensor::ones(1f32, [1, 4], &device).unwrap();
        let out = sae.forward_matryoshka(&input, 0.25).unwrap();
        // 25% of 8 hidden = 2 active dims → output shape [1, 4]
        assert_eq!(out.shape().dims(), &[1, 4]);
    }

    #[test]
    fn test_sparsity_penalty_positive() {
        let device = Device::Cpu;
        let enc = Tensor::randn(0f32, 1f32, [4, 8], &device).unwrap();
        let dec = Tensor::randn(0f32, 1f32, [8, 4], &device).unwrap();
        let sae = MatryoshkaSAE::new(enc, dec).with_sparsity_lambda(0.1);
        let acts = Tensor::randn(0f32, 1f32, [2, 8], &device).unwrap();
        let penalty = sae.sparsity_penalty(&acts).unwrap();
        let val = penalty.to_scalar::<f32>().unwrap();
        assert!(val >= 0.0, "Sparsity penalty should be non-negative");
    }

    #[test]
    fn test_sparsity_penalty_zero_activations() {
        let device = Device::Cpu;
        let enc = Tensor::randn(0f32, 1f32, [4, 8], &device).unwrap();
        let dec = Tensor::randn(0f32, 1f32, [8, 4], &device).unwrap();
        let sae = MatryoshkaSAE::new(enc, dec).with_sparsity_lambda(0.1);
        let acts = Tensor::zeros(0f32, [2, 8], &device).unwrap();
        let penalty = sae.sparsity_penalty(&acts).unwrap();
        let val = penalty.to_scalar::<f32>().unwrap();
        // log(1+0) = 0, so penalty = 0
        assert!((val - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_scheduler_workflow() {
        // Simulate a full scheduling decision
        let fitness = 8.0;
        let energy = 3.0;
        let delay = 1.0;
        let v = 1.5;
        let resolution = MatryoshkaSAE::compute_drift_plus_penalty(fitness, energy, delay, v);
        // utility = 8 - 1.5*(3+1) = 8 - 6 = 2 → core
        assert_eq!(resolution, 0.25);

        let matryoshka_res: MatryoshkaResolution = resolution.into();
        assert_eq!(matryoshka_res, MatryoshkaResolution::Core);
        assert_eq!(matryoshka_res.fraction(), 0.25);
    }
}
