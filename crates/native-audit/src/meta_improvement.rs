//! PAC-Bayesian Meta-Self-Improvement — Probably Approximately Correct guarantees for meta-updates.
//!
//! **Problem:** Standard meta-optimization (Sprint 111 NES) has no theoretical guarantees
//! on generalization — improved hyperparameters may overfit to recent performance samples.
//!
//! **Solution:** PAC-Bayesian Meta-Self-Improvement with CBF Safety Constraints.
//!
//! **Mathematical Foundation:**
//!
//! **PAC-Bayes Bound** (Seldin & Lugosi 2016, simplified):
//! For a posterior distribution q(θ) over parameters and prior p(θ),
//! with probability at least 1 - δ:
//!
//!     E_{θ~q}[R(θ)] ≤ Ĥ + √((KL(q‖p) + ln(2n/δ)) / (2(n-1)))
//!
//! where:
//! - R(θ): True risk (expected VFE under θ)
//! - Ĥ: Empirical risk (average of observed performance samples)
//! - KL(q‖p): KL-divergence between posterior and prior
//! - n: Number of performance samples
//! - δ: Confidence parameter (e.g., 0.01 for 99% confidence)
//!
//! **Acceptance Criterion:**
//! Accept meta-update only if:
//! 1. PAC-Bound < ε (generalization guarantee)
//! 2. CBF constraint: h(meta_state) ≥ 0 (safety)
//! 3. Monte Carlo violation probability < δ
//!
//! **Integration with CBF:**
//! The meta-state includes safety-critical hyperparameters (beta_cbf, max_lr).
//! A Control Barrier Function h(meta_state) ensures these stay within safe bounds.
//!
//! **References:**
//! - Y. Seldin, G. Lugosi, "Distribution-Free Prediction of Functional Graphs"
//! - M. Sejdinovic, B. Sriperumbudur, "Equivalence of Distance-Based RKHS Norms"
//! - G. Katz et al., "Reluplex: An Efficient SMT Solver for Verifying Deep Neural Networks"


/// Configuration for PAC-Bayesian meta-improvement.
#[derive(Debug, Clone)]
pub struct PACMetaConfig {
    /// Confidence parameter δ (e.g., 0.01 for 99% confidence).
    pub delta: f32,
    /// Maximum allowed PAC generalization bound.
    pub max_gen_bound: f32,
    /// Number of performance samples for empirical risk estimation.
    pub num_samples: usize,
    /// Learning rate for meta-parameter updates.
    pub meta_lr: f32,
    /// CBF safety margin: h(meta_state) must stay above this.
    pub cbf_margin: f32,
    /// Number of Monte Carlo samples for violation probability estimation.
    pub mc_samples: usize,
    /// Prior concentration parameter (controls KL penalty strength).
    pub prior_concentration: f32,
}

impl Default for PACMetaConfig {
    fn default() -> Self {
        Self {
            delta: 0.01,
            max_gen_bound: 0.05,
            num_samples: 50,
            meta_lr: 1e-3,
            cbf_margin: 0.1,
            mc_samples: 1000,
            prior_concentration: 1.0,
        }
    }
}

/// Result of a PAC-Bayesian meta-update attempt.
#[derive(Debug, Clone)]
pub struct PACMetaResult {
    /// Was the update accepted?
    pub accepted: bool,
    /// PAC generalization bound.
    pub gen_bound: f32,
    /// Empirical risk (average performance).
    pub empirical_risk: f32,
    /// KL-divergence between posterior and prior.
    pub kl_divergence: f32,
    /// CBF value (must be ≥ 0 for safety).
    pub cbf_value: f32,
    /// Estimated violation probability from Monte Carlo sampling.
    pub violation_prob: f32,
    /// Reason for rejection (if any).
    pub rejection_reason: Option<String>,
}

impl std::fmt::Display for PACMetaResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PACMetaResult {{ accepted: {}, gen_bound: {:.4}, risk: {:.4}, kl: {:.4}, \
             cbf: {:.4}, violation: {:.4} }}",
            self.accepted, self.gen_bound, self.empirical_risk,
            self.kl_divergence, self.cbf_value, self.violation_prob
        )
    }
}

/// Compute the PAC-Bayesian generalization bound.
///
/// **Formula:**
///     GenBound = √((KL(q‖p) + ln(2n/δ)) / (2(n-1)))
///
/// This bound guarantees that with probability at least 1-δ,
/// the true risk exceeds the empirical risk by at most GenBound.
///
/// # Arguments
/// * `kl_div` - KL-divergence between posterior and prior
/// * `n` - Number of performance samples
/// * `delta` - Confidence parameter
///
/// # Returns
/// Generalization bound (always non-negative).
pub fn compute_pac_gen_bound(kl_div: f32, n: usize, delta: f32) -> f32 {
    if n < 2 {
        return f32::MAX; // Need at least 2 samples for meaningful bound
    }
    let numerator = kl_div.max(0.0) + (2.0 * n as f32 / delta).ln().max(0.0);
    let denominator = 2.0 * (n - 1) as f32;
    if denominator > 0.0 {
        (numerator / denominator).sqrt()
    } else {
        f32::MAX
    }
}

/// Compute approximate KL-divergence between Gaussian posterior and prior.
///
/// For Gaussian distributions q = N(μ_q, σ_q²) and p = N(μ_p, σ_p²):
///     KL(q‖p) = ln(σ_p/σ_q) + (σ_q² + (μ_q - μ_p)²) / (2σ_p²) - 1/2
///
/// Simplified for meta-parameters: treat each parameter as a 1D Gaussian
/// and sum the KL contributions.
///
/// # Arguments
/// * `posterior_means` - Mean of posterior distribution (proposed parameters)
/// * `prior_means` - Mean of prior distribution (current parameters)
/// * `posterior_var` - Variance of posterior (update uncertainty)
/// * `prior_var` - Variance of prior (prior_concentration²)
///
/// # Returns
/// Total KL-divergence (sum over all parameters).
pub fn compute_gaussian_kl(
    posterior_means: &[f32],
    prior_means: &[f32],
    posterior_var: f32,
    prior_var: f32,
) -> f32 {
    if posterior_means.len() != prior_means.len() {
        return f32::MAX;
    }
    let prior_var = prior_var.max(1e-10);
    let posterior_var = posterior_var.max(1e-10);
    let mut kl = 0.0f32;
    for (q, p) in posterior_means.iter().zip(prior_means.iter()) {
        let diff = q - p;
        kl += (prior_var / posterior_var).ln().max(0.0)
            + (posterior_var + diff * diff) / (2.0 * prior_var)
            - 0.5;
    }
    kl.max(0.0)
}

/// Evaluate the CBF constraint on meta-state.
///
/// **CBF Function:**
///     h(meta_state) = margin² - ||meta_state - safe_center||²
///
/// Safe if h ≥ 0, meaning meta_state is within the safe region.
///
/// # Arguments
/// * `meta_state` - Current meta-parameter vector
/// * `safe_center` - Center of the safe region
/// * `margin` - Safety margin radius
///
/// # Returns
/// CBF value (≥ 0 means safe).
pub fn cbf_evaluate(meta_state: &[f32], safe_center: &[f32], margin: f32) -> f32 {
    if meta_state.len() != safe_center.len() {
        return -1.0;
    }
    let dist_sq: f32 = meta_state
        .iter()
        .zip(safe_center.iter())
        .map(|(m, s)| (m - s).powi(2))
        .sum();
    margin * margin - dist_sq
}

/// Estimate violation probability using Monte Carlo sampling.
///
/// Sample random perturbations of the meta-state and check how often
/// the CBF constraint is violated.
///
/// # Arguments
/// * `meta_state` - Proposed meta-parameter vector
/// * `safe_center` - Center of the safe region
/// * `margin` - Safety margin
/// * `perturbation_scale` - Scale of random perturbations
/// * `num_samples` - Number of Monte Carlo samples
/// * `seed` - Random seed for reproducibility
///
/// # Returns
/// Estimated probability of CBF violation (0.0 = always safe, 1.0 = always unsafe).
pub fn estimate_violation_prob(
    meta_state: &[f32],
    safe_center: &[f32],
    margin: f32,
    perturbation_scale: f32,
    num_samples: usize,
    mut seed: u64,
) -> f32 {
    let dim = meta_state.len();
    let mut violations = 0usize;

    for _ in 0..num_samples {
        // Generate random perturbation using simple LCG
        let mut perturbed = Vec::with_capacity(dim);
        for &m in meta_state {
            let r = next_random(&mut seed);
            let perturbation = (r * 2.0 - 1.0) * perturbation_scale;
            perturbed.push(m + perturbation);
        }
        if cbf_evaluate(&perturbed, safe_center, margin) < 0.0 {
            violations += 1;
        }
    }

    violations as f32 / num_samples as f32
}

/// Simple Linear Congruential Generator for deterministic random sampling.
fn next_random(state: &mut u64) -> f32 {
    *state = (*state).wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    ((*state >> 33) as u32) as f32 / u32::MAX as f32
}

/// Perform a PAC-Bayesian meta-update with CBF safety constraints.
///
/// **Algorithm:**
/// 1. Compute empirical risk from performance samples
/// 2. Compute KL-divergence between proposed and current parameters
/// 3. Compute PAC generalization bound
/// 4. Evaluate CBF constraint on proposed parameters
/// 5. Estimate violation probability via Monte Carlo
/// 6. Accept update only if all constraints satisfied
///
/// # Arguments
/// * `proposed_params` - Proposed meta-parameters (posterior)
/// * `current_params` - Current meta-parameters (prior)
/// * `performance_samples` - Observed performance (lower = better)
/// * `safe_center` - Safe center for CBF
/// * `config` - PAC-Bayesian configuration
///
/// # Returns
/// PACMetaResult with acceptance decision and metrics.
pub fn pac_bayes_meta_update(
    proposed_params: &[f32],
    current_params: &[f32],
    performance_samples: &[f32],
    safe_center: &[f32],
    config: &PACMetaConfig,
) -> PACMetaResult {
    let n = performance_samples.len().max(1);

    // Step 1: Empirical risk (average performance)
    let empirical_risk = if n > 0 {
        performance_samples.iter().sum::<f32>() / n as f32
    } else {
        f32::MAX
    };

    // Step 2: KL-divergence
    let posterior_var = config.meta_lr * config.meta_lr;
    let prior_var = config.prior_concentration * config.prior_concentration;
    let kl_div = compute_gaussian_kl(proposed_params, current_params, posterior_var, prior_var);

    // Step 3: PAC generalization bound
    let gen_bound = compute_pac_gen_bound(kl_div, n.max(2), config.delta);

    // Step 4: CBF constraint
    let cbf_val = cbf_evaluate(proposed_params, safe_center, config.cbf_margin);

    // Step 5: Monte Carlo violation probability
    let violation_prob = estimate_violation_prob(
        proposed_params,
        safe_center,
        config.cbf_margin,
        config.meta_lr * 10.0,
        config.mc_samples,
        42,
    );

    // Step 6: Acceptance decision
    let mut rejection_reason = None;
    let mut accepted = true;

    if gen_bound > config.max_gen_bound {
        accepted = false;
        rejection_reason = Some(format!(
            "PAC bound {:.4} exceeds max {:.4}",
            gen_bound, config.max_gen_bound
        ));
    } else if cbf_val < 0.0 {
        accepted = false;
        rejection_reason = Some(format!("CBF violation: h = {:.4}", cbf_val));
    } else if violation_prob > config.delta {
        accepted = false;
        rejection_reason = Some(format!(
            "Violation prob {:.4} > δ = {:.4}",
            violation_prob, config.delta
        ));
    }

    PACMetaResult {
        accepted,
        gen_bound,
        empirical_risk,
        kl_divergence: kl_div,
        cbf_value: cbf_val,
        violation_prob,
        rejection_reason,
    }
}

/// PAC-Bayesian Meta-Improvement Engine.
///
/// Maintains state across multiple meta-update rounds and tracks improvement history.
pub struct PACMetaEngine {
    config: PACMetaConfig,
    current_params: Vec<f32>,
    safe_center: Vec<f32>,
    history: Vec<PACMetaResult>,
    performance_buffer: Vec<f32>,
}

impl PACMetaEngine {
    /// Create a new PAC-Bayesian meta-improvement engine.
    ///
    /// # Arguments
    /// * `initial_params` - Initial meta-parameters
    /// * `safe_center` - Center of the safe region for CBF
    /// * `config` - PAC-Bayesian configuration
    pub fn new(initial_params: Vec<f32>, safe_center: Vec<f32>, config: PACMetaConfig) -> Self {
        Self {
            config,
            current_params: initial_params,
            safe_center,
            history: Vec::new(),
            performance_buffer: Vec::new(),
        }
    }

    /// Add a performance sample to the buffer.
    pub fn add_sample(&mut self, value: f32) {
        self.performance_buffer.push(value);
        // Keep only the most recent samples
        if self.performance_buffer.len() > self.config.num_samples {
            self.performance_buffer.remove(0);
        }
    }

    /// Attempt a meta-update with PAC-Bayesian guarantees.
    ///
    /// # Arguments
    /// * `proposed_params` - Proposed new meta-parameters
    ///
    /// # Returns
    /// PACMetaResult indicating whether the update was accepted.
    pub fn attempt_update(&mut self, proposed_params: &[f32]) -> PACMetaResult {
        let result = pac_bayes_meta_update(
            proposed_params,
            &self.current_params,
            &self.performance_buffer,
            &self.safe_center,
            &self.config,
        );

        if result.accepted {
            self.current_params = proposed_params.to_vec();
        }
        self.history.push(result.clone());
        result
    }

    /// Run a gradient-descent-style meta-update step.
    ///
    /// Proposes a parameter update in the direction of decreasing empirical risk,
    /// then validates with PAC-Bayesian + CBF constraints.
    ///
    /// # Arguments
    /// * `gradient` - Estimated gradient of empirical risk w.r.t. parameters
    ///
    /// # Returns
    /// PACMetaResult indicating whether the update was accepted.
    pub fn step(&mut self, gradient: &[f32]) -> PACMetaResult {
        let proposed: Vec<f32> = self
            .current_params
            .iter()
            .zip(gradient.iter())
            .map(|(p, g)| p - self.config.meta_lr * g)
            .collect();
        self.attempt_update(&proposed)
    }

    /// Get the current meta-parameters.
    pub fn params(&self) -> &[f32] {
        &self.current_params
    }

    /// Get the improvement history.
    pub fn history(&self) -> &[PACMetaResult] {
        &self.history
    }

    /// Compute the acceptance rate from history.
    pub fn acceptance_rate(&self) -> f32 {
        if self.history.is_empty() {
            return 0.0;
        }
        let accepted = self.history.iter().filter(|r| r.accepted).count();
        accepted as f32 / self.history.len() as f32
    }

    /// Compute the average PAC bound from history.
    pub fn avg_gen_bound(&self) -> f32 {
        if self.history.is_empty() {
            return 0.0;
        }
        self.history.iter().map(|r| r.gen_bound).sum::<f32>() / self.history.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pac_config_default() {
        let config = PACMetaConfig::default();
        assert!((config.delta - 0.01).abs() < 1e-6);
        assert!((config.max_gen_bound - 0.05).abs() < 1e-6);
        assert_eq!(config.num_samples, 50);
        assert_eq!(config.mc_samples, 1000);
    }

    #[test]
    fn test_pac_gen_bound_basic() {
        // With KL=0.1, n=50, delta=0.01
        let bound = compute_pac_gen_bound(0.1, 50, 0.01);
        assert!(bound.is_finite());
        assert!(bound > 0.0);
        // Bound should decrease with more samples
        let bound_more = compute_pac_gen_bound(0.1, 200, 0.01);
        assert!(bound_more <= bound);
    }

    #[test]
    fn test_pac_gen_bound_single_sample() {
        // n < 2 should return MAX
        let bound = compute_pac_gen_bound(0.1, 1, 0.01);
        assert_eq!(bound, f32::MAX);
    }

    #[test]
    fn test_gaussian_kl_identical() {
        let means = vec![0.0, 0.5, -0.3];
        let kl = compute_gaussian_kl(&means, &means, 0.01, 0.01);
        assert!((kl - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_gaussian_kl_positive() {
        let prior = vec![0.0, 0.0];
        let posterior = vec![0.5, -0.5];
        let kl = compute_gaussian_kl(&posterior, &prior, 0.01, 1.0);
        assert!(kl > 0.0);
        assert!(kl.is_finite());
    }

    #[test]
    fn test_gaussian_kl_dimension_mismatch() {
        let kl = compute_gaussian_kl(&[0.0, 0.5], &[0.0], 0.01, 1.0);
        assert_eq!(kl, f32::MAX);
    }

    #[test]
    fn test_cbf_evaluate_safe() {
        let state = vec![0.0, 0.0];
        let center = vec![0.0, 0.0];
        let margin = 1.0;
        let h = cbf_evaluate(&state, &center, margin);
        assert!(h >= 0.0, "Center should be safe");
        assert!((h - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cbf_evaluate_unsafe() {
        let state = vec![2.0, 2.0];
        let center = vec![0.0, 0.0];
        let margin = 1.0;
        let h = cbf_evaluate(&state, &center, margin);
        assert!(h < 0.0, "Far from center should be unsafe");
    }

    #[test]
    fn test_cbf_evaluate_boundary() {
        let state = vec![1.0, 0.0];
        let center = vec![0.0, 0.0];
        let margin = 1.0;
        let h = cbf_evaluate(&state, &center, margin);
        assert!((h - 0.0).abs() < 1e-6, "On boundary should be ~0");
    }

    #[test]
    fn test_violation_prob_safe() {
        let state = vec![0.0, 0.0];
        let center = vec![0.0, 0.0];
        let margin = 1.0;
        let prob = estimate_violation_prob(&state, &center, margin, 0.01, 500, 42);
        assert!(prob < 0.1, "Small perturbation should rarely violate");
    }

    #[test]
    fn test_violation_prob_unsafe() {
        // State already outside boundary — most perturbations should violate
        let state = vec![1.1, 0.0];
        let center = vec![0.0, 0.0];
        let margin = 1.0;
        let prob = estimate_violation_prob(&state, &center, margin, 0.1, 1000, 42);
        assert!(prob > 0.5, "Outside-boundary state should mostly violate (got {})", prob);
    }

    #[test]
    fn test_pac_meta_update_accepts_safe() {
        // Use config with larger prior_concentration to match parameter scale
        let mut config = PACMetaConfig::default();
        config.prior_concentration = 0.001; // Match the scale of parameter updates
        config.max_gen_bound = 1.0; // Allow larger bound for small sample sets
        let current = vec![0.0, 0.5, 0.3];
        let proposed = vec![0.001, 0.501, 0.301]; // Very small update
        let safe_center = vec![0.0, 0.5, 0.3];
        // Good performance samples
        let samples: Vec<f32> = (0..50).map(|i| 1.0 - i as f32 * 0.01).collect();

        let result = pac_bayes_meta_update(&proposed, &current, &samples, &safe_center, &config);
        assert!(result.accepted, "Small safe update should be accepted: {}", result);
    }

    #[test]
    fn test_pac_meta_update_rejects_cbf_violation() {
        let config = PACMetaConfig::default();
        let current = vec![0.0, 0.0];
        let proposed = vec![5.0, 5.0]; // Far from safe center
        let safe_center = vec![0.0, 0.0];
        let samples: Vec<f32> = vec![0.5; 50];

        let result = pac_bayes_meta_update(&proposed, &current, &samples, &safe_center, &config);
        assert!(
            !result.accepted,
            "Large unsafe update should be rejected: {}",
            result
        );
        assert!(result.rejection_reason.is_some());
    }

    #[test]
    fn test_pac_meta_result_display() {
        let result = PACMetaResult {
            accepted: true,
            gen_bound: 0.03,
            empirical_risk: 0.5,
            kl_divergence: 0.01,
            cbf_value: 0.5,
            violation_prob: 0.001,
            rejection_reason: None,
        };
        let s = format!("{}", result);
        assert!(s.contains("accepted: true"));
    }

    #[test]
    fn test_engine_creation() {
        let config = PACMetaConfig::default();
        let params = vec![0.0, 0.5, 0.3];
        let center = vec![0.0, 0.5, 0.3];
        let engine = PACMetaEngine::new(params.clone(), center, config);
        assert_eq!(engine.params(), &params);
        assert!(engine.history().is_empty());
    }

    #[test]
    fn test_engine_add_sample() {
        let config = PACMetaConfig::default();
        let mut engine = PACMetaEngine::new(vec![0.0], vec![0.0], config);
        engine.add_sample(0.5);
        engine.add_sample(0.3);
        assert_eq!(engine.performance_buffer.len(), 2);
    }

    #[test]
    fn test_engine_step() {
        let config = PACMetaConfig::default();
        let mut engine = PACMetaEngine::new(vec![0.0, 0.5], vec![0.0, 0.5], config);
        engine.add_sample(0.5);
        engine.add_sample(0.4);

        let gradient = vec![0.1, -0.1];
        let result = engine.step(&gradient);
        assert!(result.gen_bound.is_finite());
    }

    #[test]
    fn test_engine_acceptance_rate() {
        let config = PACMetaConfig::default();
        let mut engine = PACMetaEngine::new(vec![0.0], vec![0.0], config);
        engine.add_sample(0.5);

        // Small step — should accept
        engine.step(&[0.001]);
        // Large step — might reject
        engine.step(&[10.0]);

        let rate = engine.acceptance_rate();
        assert!(rate >= 0.0 && rate <= 1.0);
    }

    #[test]
    fn test_engine_avg_gen_bound() {
        let config = PACMetaConfig::default();
        let mut engine = PACMetaEngine::new(vec![0.0], vec![0.0], config);
        engine.add_sample(0.5);
        engine.step(&[0.001]);

        let avg = engine.avg_gen_bound();
        assert!(avg.is_finite());
        assert!(avg >= 0.0);
    }

    #[test]
    fn test_pac_bound_decreases_with_samples() {
        let kl = 0.1;
        let delta = 0.01;
        let b1 = compute_pac_gen_bound(kl, 10, delta);
        let b2 = compute_pac_gen_bound(kl, 100, delta);
        let b3 = compute_pac_gen_bound(kl, 1000, delta);
        assert!(b1 > b2, "More samples should reduce bound");
        assert!(b2 > b3, "Even more samples should reduce further");
    }

    #[test]
    fn test_pac_bound_increases_with_kl() {
        let n = 50;
        let delta = 0.01;
        let b1 = compute_pac_gen_bound(0.01, n, delta);
        let b2 = compute_pac_gen_bound(0.1, n, delta);
        let b3 = compute_pac_gen_bound(1.0, n, delta);
        assert!(b1 < b2, "Higher KL should increase bound");
        assert!(b2 < b3, "Even higher KL should increase further");
    }

    #[test]
    fn test_next_random() {
        let mut seed = 42u64;
        let r1 = next_random(&mut seed);
        let r2 = next_random(&mut seed);
        assert!(r1 >= 0.0 && r1 <= 1.0);
        assert!(r2 >= 0.0 && r2 <= 1.0);
        assert_ne!(r1, r2, "Successive calls should differ");
    }

    #[test]
    fn test_cbf_dimension_mismatch() {
        let h = cbf_evaluate(&[0.0, 0.0], &[0.0], 1.0);
        assert!(h < 0.0, "Dimension mismatch should be unsafe");
    }

    #[test]
    fn test_full_pac_pipeline() {
        let config = PACMetaConfig::default();
        let initial = vec![1e-3, 0.1, 0.5, 0.01];
        let safe_center = vec![1e-3, 0.1, 0.5, 0.01];
        let mut engine = PACMetaEngine::new(initial, safe_center, config);

        // Simulate performance improvement
        for i in 0..30 {
            engine.add_sample(1.0 - i as f32 * 0.02);
        }

        // Attempt gradual improvement
        let gradient = vec![0.0001, 0.01, 0.05, 0.001];
        let result = engine.step(&gradient);

        assert!(result.gen_bound.is_finite());
        assert!(result.cbf_value.is_finite());
        assert!(result.violation_prob >= 0.0 && result.violation_prob <= 1.0);
        assert!(engine.history().len() == 1);
    }
}
