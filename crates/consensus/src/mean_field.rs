//! McKean-Vlasov SDE — Symbiotic Mean-Field Particle Dynamics.
//!
//! **Sprint 145:** Implements the McKean-Vlasov stochastic differential equation
//! for symbiotic mean-field consensus across distributed nodes. Each particle
//! (node) evolves under the influence of its own VFE gradient, the population
//! mean field (gossip-aggregated belief), and a barrier function for safety.
//!
//! **McKean-Vlasov SDE:**
//! ```math
//! dX^i_t = b(X^i_t, μ_t) dt + σ dW^i_t
//! ```
//!
//! **Drift decomposition:**
//! ```math
//! b(X, μ) = f_VFE(X) + η·C(X, μ) - δ·B(X)
//! ```
//!
//! Where:
//! - `f_VFE(X)`: Variational Free Energy gradient (local optimization)
//! - `C(X, μ) = α(X - μ)`: Mean-field coupling (attraction to population mean)
//! - `B(X)`: Barrier function gradient (safety enforcement)
//! - `σ`: Diffusion coefficient (exploration noise)
//! - `W^i_t`: Independent Wiener processes per particle
//!
//! **Euler-Maruyama discretization:**
//! ```math
//! X^i_{t+1} = X^i_t + b(X^i_t, μ_t)·Δt + σ·√(Δt)·ξ^i
//! ```
//! where `ξ^i ~ N(0, I)` i.i.d. standard normal.

use rand::{Rng, SeedableRng, rngs::StdRng};

/// Configuration for McKean-Vlasov SDE particle dynamics.
#[derive(Debug, Clone)]
pub struct MeanFieldConfig {
    /// Mean-field coupling strength (η): attraction to population mean.
    pub eta: f64,
    /// Barrier function weight (δ): safety enforcement intensity.
    pub delta: f64,
    /// Diffusion coefficient (σ): exploration noise magnitude.
    pub sigma: f64,
    /// Time step (Δt): Euler-Maruyama integration step.
    pub dt: f64,
    /// Safety boundary radius: states beyond this trigger barrier.
    pub safety_radius: f64,
    /// Random seed for reproducibility.
    pub seed: u64,
}

impl Default for MeanFieldConfig {
    fn default() -> Self {
        Self {
            eta: 0.5,
            delta: 0.3,
            sigma: 0.1,
            dt: 0.01,
            safety_radius: 10.0,
            seed: 42,
        }
    }
}

impl MeanFieldConfig {
    /// Fast convergence — strong coupling, low noise.
    pub fn fast() -> Self {
        Self {
            eta: 1.0,
            delta: 0.5,
            sigma: 0.02,
            dt: 0.05,
            ..Self::default()
        }
    }

    /// High exploration — weak coupling, high noise.
    pub fn exploratory() -> Self {
        Self {
            eta: 0.1,
            delta: 0.1,
            sigma: 0.3,
            dt: 0.005,
            ..Self::default()
        }
    }

    /// Conservative — weak coupling, minimal noise, tight safety.
    pub fn conservative() -> Self {
        Self {
            eta: 0.2,
            delta: 0.8,
            sigma: 0.01,
            dt: 0.001,
            safety_radius: 5.0,
            ..Self::default()
        }
    }

    /// Set coupling strength.
    pub fn with_eta(mut self, eta: f64) -> Self {
        self.eta = eta.max(0.0).min(2.0);
        self
    }

    /// Set barrier weight.
    pub fn with_delta(mut self, delta: f64) -> Self {
        self.delta = delta.max(0.0).min(2.0);
        self
    }

    /// Set diffusion coefficient.
    pub fn with_sigma(mut self, sigma: f64) -> Self {
        self.sigma = sigma.max(0.0).min(1.0);
        self
    }

    /// Set time step.
    pub fn with_dt(mut self, dt: f64) -> Self {
        self.dt = dt.max(0.000_1).min(0.1);
        self
    }

    /// Set safety radius.
    pub fn with_safety_radius(mut self, radius: f64) -> Self {
        self.safety_radius = radius.max(1.0);
        self
    }

    /// Set random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }
}

/// Result of a single McKean-Vlasov particle step.
#[derive(Debug, Clone)]
pub struct MeanFieldStepResult {
    /// Updated particle positions after the step.
    pub particles: Vec<Vec<f64>>,
    /// Population mean field μ_t (empirical mean).
    pub mean_field: Vec<f64>,
    /// Empirical covariance trace (dispersion measure).
    pub dispersion: f64,
    /// Total VFE drift magnitude across particles.
    pub vfe_drift_magnitude: f64,
    /// Total barrier activation magnitude.
    pub barrier_magnitude: f64,
    /// Number of particles that triggered safety barrier.
    pub barrier_activations: usize,
}

impl std::fmt::Display for MeanFieldStepResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MeanFieldStep {{ particles: {}, dim: {}, dispersion: {:.6}, barrier_activations: {} }}",
            self.particles.len(),
            if self.particles.is_empty() {
                0
            } else {
                self.particles[0].len()
            },
            self.dispersion,
            self.barrier_activations
        )
    }
}

/// Compute the empirical mean field μ_t from particle positions.
///
/// ```math
/// μ_t = (1/N) Σ_i X^i_t
/// ```
pub fn compute_mean_field(particles: &[Vec<f64>]) -> Vec<f64> {
    let n = particles.len();
    if n == 0 {
        return vec![];
    }
    let dim = particles[0].len();
    let mut mean = vec![0.0; dim];
    for particle in particles {
        for (m, &x) in mean.iter_mut().zip(particle.iter()) {
            *m += x;
        }
    }
    for m in &mut mean {
        *m /= n as f64;
    }
    mean
}

/// Compute the empirical dispersion (trace of covariance matrix).
///
/// ```math
/// dispersion = (1/N) Σ_i ||X^i_t - μ_t||²
/// ```
pub fn compute_dispersion(particles: &[Vec<f64>], mean_field: &[f64]) -> f64 {
    let mut total = 0.0;
    for particle in particles {
        for (&x, &m) in particle.iter().zip(mean_field.iter()) {
            let diff = x - m;
            total += diff * diff;
        }
    }
    total / particles.len() as f64
}

/// Compute VFE-based drift for a single particle.
///
/// Uses a quadratic energy landscape: `f_VFE(X) = -X` (gradient descent toward origin).
/// In production, this would be replaced with actual VFE gradient from the audit kernel.
///
/// ```math
/// f_VFE(X) = -∇E(X) = -X
/// ```
pub fn vfe_drift(x: &[f64]) -> Vec<f64> {
    x.iter().map(|&v| -v).collect()
}

/// Compute mean-field coupling: attraction toward population mean.
///
/// ```math
/// C(X, μ) = η · (μ - X)
/// ```
pub fn mean_field_coupling(x: &[f64], mean_field: &[f64], eta: f64) -> Vec<f64> {
    x.iter()
        .zip(mean_field.iter())
        .map(|(&xi, &mu)| eta * (mu - xi))
        .collect()
}

/// Compute barrier function gradient for safety enforcement.
///
/// Uses a logarithmic barrier: `B(X) = -log(R² - ||X||²)` when `||X|| < R`.
/// Gradient: `∇B(X) = 2X / (R² - ||X||²)`.
/// When `||X|| >= R`, the barrier saturates to push strongly inward.
///
/// ```math
/// ∇B(X) = 2X / (R² - ||X||²)    if ||X|| < R
/// ∇B(X) = 2X / ε                if ||X|| >= R  (clamped)
/// ```
pub fn barrier_gradient(x: &[f64], safety_radius: f64) -> (Vec<f64>, bool) {
    let norm_sq: f64 = x.iter().map(|&v| v * v).sum();
    let r_sq = safety_radius * safety_radius;

    if norm_sq < r_sq {
        let denom = r_sq - norm_sq;
        let scale = 2.0 / denom.max(1e-6); // Clamp to avoid division by zero
        (x.iter().map(|&v| scale * v).collect(), false)
    } else {
        // Outside safety boundary — strong inward push
        let scale = 2.0 / 1e-6;
        (x.iter().map(|&v| scale * v).collect(), true)
    }
}

/// Compute the full drift: `b(X, μ) = f_VFE(X) + η·C(X, μ) - δ·B(X)`.
pub fn compute_drift(
    x: &[f64],
    mean_field: &[f64],
    config: &MeanFieldConfig,
) -> (Vec<f64>, bool) {
    let vfe = vfe_drift(x);
    let coupling = mean_field_coupling(x, mean_field, config.eta);
    let (barrier, activated) = barrier_gradient(x, config.safety_radius);

    let mut drift = vec![0.0; x.len()];
    for (i, (&v, &c)) in vfe.iter().zip(coupling.iter()).enumerate() {
        drift[i] = v + c - config.delta * barrier[i];
    }
    (drift, activated)
}

/// Generate a standard normal sample using Box-Muller transform.
fn box_muller_normal(rng: &mut impl Rng) -> f64 {
    // Box-Muller transform: z = sqrt(-2*ln(u1)) * cos(2*pi*u2)
    let u1: f64 = rng.gen_range(1e-10_f64..1.0_f64); // Avoid log(0)
    let u2: f64 = rng.gen_range(0.0_f64..1.0_f64);
    let radius = (-2.0_f64 * u1.ln()).sqrt();
    let angle = std::f64::consts::TAU * u2;
    radius * angle.cos()
}

/// Execute a single Euler-Maruyama step for all particles.
///
/// ```math
/// X^i_{t+1} = X^i_t + b(X^i_t, μ_t)·Δt + σ·√(Δt)·ξ^i
/// ```
pub fn mean_field_step(
    particles: &[Vec<f64>],
    config: &MeanFieldConfig,
) -> MeanFieldStepResult {
    let n = particles.len();
    if n == 0 {
        return MeanFieldStepResult {
            particles: vec![],
            mean_field: vec![],
            dispersion: 0.0,
            vfe_drift_magnitude: 0.0,
            barrier_magnitude: 0.0,
            barrier_activations: 0,
        };
    }

    let dim = particles[0].len();
    let mean = compute_mean_field(particles);
    let noise_scale = config.sigma * (config.dt).sqrt();

    let mut next_particles = Vec::with_capacity(n);
    let mut total_vfe_drift = 0.0;
    let mut total_barrier = 0.0;
    let mut barrier_count = 0usize;
    let mut rng = StdRng::seed_from_u64(config.seed);

    for particle in particles {
        let (drift, activated) = compute_drift(particle, &mean, config);

        // Accumulate statistics
        let vfe = vfe_drift(particle);
        let vfe_norm_sq: f64 = vfe.iter().map(|&v| v * v).sum();
        total_vfe_drift += vfe_norm_sq.sqrt();

        if activated {
            barrier_count += 1;
            let barrier = barrier_gradient(particle, config.safety_radius).0;
            let b_norm_sq: f64 = barrier.iter().map(|&v| v * v).sum();
            total_barrier += b_norm_sq.sqrt();
        }

        // Euler-Maruyama: X + drift*dt + sigma*sqrt(dt)*xi
        let mut next = Vec::with_capacity(dim);
        for (&x, &d) in particle.iter().zip(drift.iter()) {
            let noise = box_muller_normal(&mut rng);
            next.push(x + d * config.dt + noise_scale * noise);
        }
        next_particles.push(next);
    }

    let dispersion = compute_dispersion(&next_particles, &mean);

    MeanFieldStepResult {
        particles: next_particles,
        mean_field: mean,
        dispersion,
        vfe_drift_magnitude: total_vfe_drift,
        barrier_magnitude: total_barrier,
        barrier_activations: barrier_count,
    }
}

/// Run multiple McKean-Vlasov steps, returning the trajectory.
pub fn mean_field_trajectory(
    initial_particles: &[Vec<f64>],
    config: &MeanFieldConfig,
    steps: usize,
) -> Vec<MeanFieldStepResult> {
    let mut current = initial_particles.to_vec();
    let mut trajectory = Vec::with_capacity(steps);

    for step in 0..steps {
        // Increment seed per step to avoid identical noise
        let step_config = MeanFieldConfig {
            seed: config.seed + step as u64,
            ..config.clone()
        };
        let result = mean_field_step(&current, &step_config);

        // Update particles for next iteration
        current = result.particles.clone();
        trajectory.push(result);
    }

    trajectory
}

/// Verify that the mean-field dynamics preserve the safety invariant:
/// all particles remain within the safety boundary (with high probability).
pub fn verify_safety_invariant(
    particles: &[Vec<f64>],
    config: &MeanFieldConfig,
) -> (bool, usize) {
    let mut violations = 0;
    let r_sq = config.safety_radius * config.safety_radius;
    for particle in particles {
        let norm_sq: f64 = particle.iter().map(|&v| v * v).sum();
        if norm_sq > r_sq {
            violations += 1;
        }
    }
    (violations == 0, violations)
}

// ---------------------------------------------------------------------------
// S147 — Graphon Mean-Field Drift
// ---------------------------------------------------------------------------

/// Graphon Mean-Field Drift (S147).
///
/// **Mathematical Formula:**
/// ```math
/// b(X, μ) = -∇VFE(X) + η·∫W(X,Y)(Y-X)μ(dY) - δ·∇B(X)
/// ```
///
/// Where:
/// - `-∇VFE(X)`: Negative VFE gradient (local optimization, descent toward origin).
/// - `η·∫W(X,Y)(Y-X)μ(dY)`: Graphon-weighted mean-field coupling.
///   Integral approximated empirically via peer sampling:
///   `∫W(X,Y)(Y-X)μ(dY) ≈ (1/N)·Σ_i W(X, Y_i)·(Y_i - X)`
/// - `δ·∇B(X)`: Barrier function gradient (safety enforcement).
/// - `W(X,Y) = max(0, cosine_similarity(X, Y))`: Cosine similarity graphon kernel.
///
/// # Arguments
/// * `x_state` - Current particle state.
/// * `grad_vfe` - VFE gradient at X (negative for descent).
/// * `grad_barrier` - Barrier function gradient at X.
/// * `peer_states` - States of neighboring particles (empirical measure μ).
/// * `eta` - Graphon coupling strength.
/// * `delta` - Barrier weight.
pub fn compute_graphon_drift(
    x_state: &[f64],
    grad_vfe: &[f64],
    grad_barrier: &[f64],
    peer_states: &[Vec<f64>],
    eta: f64,
    delta: f64,
) -> Vec<f64> {
    let dim = x_state.len();

    // 1. Graphon-weighted interaction: Σ W(X, Y_i) · (Y_i - X)
    let mut interaction_sum = vec![0.0f64; dim];
    for peer in peer_states {
        let w_xy = compute_graphon_kernel(x_state, peer);
        for (acc, (&y, &x)) in interaction_sum.iter_mut().zip(peer.iter().zip(x_state.iter())) {
            *acc += w_xy * (y - x);
        }
    }

    // 2. Empirical integral approximation: (1/N) · Σ
    let n_peers = peer_states.len().max(1) as f64;
    let integral_term = interaction_sum.iter().map(|&v| v / n_peers).collect::<Vec<f64>>();

    // 3. Assemble drift: b(X, μ) = -∇VFE(X) + η·integral - δ·∇B(X)
    let mut drift = vec![0.0f64; dim];
    for (i, (&vfe, &integral)) in grad_vfe.iter().zip(integral_term.iter()).enumerate() {
        drift[i] = -vfe + eta * integral - delta * grad_barrier[i];
    }

    drift
}

/// Graphon Kernel: Cosine similarity with ReLU clipping (S147).
///
/// ```math
/// W(X, Y) = max(0, cos(X, Y)) = max(0, (X·Y) / (||X||·||Y||))
/// ```
///
/// Returns 0 if either vector is zero-norm (undefined cosine).
/// Negative cosine similarity → 0 (no repulsive coupling).
pub fn compute_graphon_kernel(x: &[f64], y: &[f64]) -> f64 {
    let dot: f64 = x.iter().zip(y.iter()).map(|(&a, &b)| a * b).sum();
    let norm_x: f64 = x.iter().map(|&v| v * v).sum::<f64>().sqrt();
    let norm_y: f64 = y.iter().map(|&v| v * v).sum::<f64>().sqrt();

    if norm_x < 1e-12 || norm_y < 1e-12 {
        return 0.0;
    }

    let cos_sim = dot / (norm_x * norm_y);
    cos_sim.max(0.0)
}

/// Graphon mean-field step using `compute_graphon_drift` instead of uniform coupling.
///
/// Replaces the standard mean-field coupling `η·(μ - X)` with the graphon-weighted
/// interaction `η·∫W(X,Y)(Y-X)μ(dY)` for heterogeneous peer coupling.
pub fn graphon_mean_field_step(
    particles: &[Vec<f64>],
    config: &MeanFieldConfig,
) -> MeanFieldStepResult {
    let n = particles.len();
    if n == 0 {
        return MeanFieldStepResult {
            particles: vec![],
            mean_field: vec![],
            dispersion: 0.0,
            vfe_drift_magnitude: 0.0,
            barrier_magnitude: 0.0,
            barrier_activations: 0,
        };
    }

    let dim = particles[0].len();
    let mut rng: StdRng = SeedableRng::seed_from_u64(config.seed);
    let sqrt_dt = config.dt.sqrt();
    let mut new_particles = Vec::with_capacity(n);
    let mut total_vfe = 0.0;
    let mut total_barrier = 0.0;
    let mut barrier_activations = 0;

    for particle in particles {
        // VFE gradient: -X (descent toward origin)
        let grad_vfe = vfe_drift(particle);

        // Barrier gradient
        let (grad_barrier, activated) = barrier_gradient(particle, config.safety_radius);
        if activated {
            barrier_activations += 1;
        }

        // Graphon drift using all other particles as peers
        let peers: Vec<Vec<f64>> = particles
            .iter()
            .filter(|&p| !p.iter().zip(particle.iter()).all(|(a, b)| (a - b).abs() < 1e-12))
            .cloned()
            .collect();

        let drift = compute_graphon_drift(
            particle,
            &grad_vfe,
            &grad_barrier,
            &peers,
            config.eta as f64,
            config.delta as f64,
        );

        // Accumulate magnitudes
        for &d in &drift {
            total_vfe += d * d;
        }
        for &b in &grad_barrier {
            total_barrier += b * b;
        }

        // Euler-Maruyama update: X_{t+1} = X_t + drift·dt + σ·√dt·ξ
        let mut new_p = Vec::with_capacity(dim);
        for &d in &drift {
            let noise = config.sigma * sqrt_dt * box_muller_normal(&mut rng);
            new_p.push(d + config.dt * d + noise);
        }
        new_particles.push(new_p);
    }

    let mean_field = compute_mean_field(&new_particles);
    let dispersion = compute_dispersion(&new_particles, &mean_field);

    MeanFieldStepResult {
        particles: new_particles,
        mean_field,
        dispersion,
        vfe_drift_magnitude: total_vfe.sqrt(),
        barrier_magnitude: total_barrier.sqrt(),
        barrier_activations,
    }
}

// ─── S149 — Sparse Mean Field Games over Graphings ────────────────────────────

/// Sparse neighbor info for graphing-based mean field.
#[derive(Debug, Clone)]
pub struct SparseNeighbor {
    /// Neighbor index.
    pub idx: usize,
    /// Neighbor state.
    pub state: Vec<f64>,
    /// Edge weight: w_{ij} = exp(-α·latency) · trust.
    pub weight: f64,
    /// Empirical measure μ_j.
    pub measure: f64,
}

/// Result of sparse mean field update.
#[derive(Debug)]
pub struct SparseMeanFieldResult {
    /// Updated particles.
    pub particles: Vec<Vec<f64>>,
    /// Sparse drift magnitudes per particle.
    pub drift_magnitudes: Vec<f64>,
    /// Total number of edges processed (sparsity metric).
    pub num_edges: usize,
    /// Total number of possible edges (for sparsity ratio).
    pub num_possible_edges: usize,
}

impl std::fmt::Display for SparseMeanFieldResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sparsity = if self.num_possible_edges > 0 {
            (1.0 - self.num_edges as f64 / self.num_possible_edges as f64) * 100.0
        } else {
            100.0
        };
        write!(f, "SparseMeanField {{ edges={}/{}, sparsity={:.1}%, avg_drift={:.4} }}",
            self.num_edges, self.num_possible_edges, sparsity,
            if self.drift_magnitudes.is_empty() {
                0.0
            } else {
                self.drift_magnitudes.iter().sum::<f64>() / self.drift_magnitudes.len() as f64
            })
    }
}

/// Sparse Mean Field Game Update over Graphings (S149).
///
/// **Mathematical Foundation:**
/// Replaces dense graphon aggregation with sparse neighbor summation over graphings
/// (local weak limit of sparse graphs, per Fabian et al. 2023-2025).
/// Suitable for libp2p power-law networks where graphon density assumption fails.
///
/// **Sparse Drift:**
/// ```math
/// b_i(x, μ) = -∇VFE_i(x) + η ∑_{j∈N(i)} w_{ij}(x_i, x_j)(x_j - x_i) μ_j
/// ```
/// where:
/// - `N(i)` — Sparse neighbor set (graphing adjacency, not dense matrix)
/// - `w_{ij} = exp(-α·latency_{ij}) · trust_{ij}` — Edge weight
/// - `μ_j` — Empirical measure at node j
///
/// **Discrete Fokker-Planck-Kolmogorov:**
/// ```math
/// μ^{t+1} ≈ μ^t - Δt ∇·(b μ) + noise
/// ```
///
/// # Arguments
/// * `particles` - Current particle states `[num_particles][dim]`.
/// * `neighbors` - Sparse neighbor list per particle (graphing adjacency).
/// * `config` - Mean field configuration.
/// * `alpha` - Latency decay parameter for edge weights.
pub fn update_sparse_mean_field(
    particles: &[Vec<f64>],
    neighbors: &[Vec<SparseNeighbor>],
    config: &MeanFieldConfig,
    _alpha: f64,
) -> SparseMeanFieldResult {
    let num_particles = particles.len();
    let dim = if num_particles > 0 { particles[0].len() } else { 0 };
    let mut new_particles = Vec::with_capacity(num_particles);
    let mut drift_magnitudes = Vec::with_capacity(num_particles);
    let mut total_edges: usize = 0;

    for i in 0..num_particles {
        let x = &particles[i];

        // 1. VFE drift: -∇VFE_i(x) ≈ -x (gradient of quadratic energy)
        let vfe_drift: Vec<f64> = x.iter().map(|v| -v * config.delta).collect();

        // 2. Sparse mean-field coupling: η ∑_{j∈N(i)} w_{ij}(x_j - x_i) μ_j
        let mut mf_coupling = vec![0.0f64; dim];
        for neighbor in &neighbors[i] {
            total_edges += 1;
            let w = neighbor.weight * neighbor.measure;
            for (d, (xj, xi)) in neighbor.state.iter().zip(x.iter()).enumerate().take(dim) {
                mf_coupling[d] += config.eta * w * (xj - xi);
            }
        }

        // 3. Barrier function: -δ ∇B(x) = -δ · x / (R² - ||x||²)
        let x_sq_norm: f64 = x.iter().map(|v| v * v).sum();
        let barrier_factor = if x_sq_norm < config.safety_radius * config.safety_radius {
            config.delta / (config.safety_radius * config.safety_radius - x_sq_norm)
        } else {
            config.delta * 10.0 // Strong barrier when outside
        };
        let barrier_drift: Vec<f64> = x.iter().map(|v| -barrier_factor * v).collect();

        // 4. Combined drift: b(x, μ) = vfe_drift + mf_coupling + barrier_drift
        let mut drift = vec![0.0f64; dim];
        for d in 0..dim {
            drift[d] = vfe_drift[d] + mf_coupling[d] + barrier_drift[d];
        }

        // 5. Euler-Maruyama step: x_{t+1} = x_t + b·Δt + σ·√(Δt)·ξ
        let noise_scale = config.sigma * (config.dt).sqrt();
        let mut new_x = Vec::with_capacity(dim);
        let mut drift_sq = 0.0f64;
        for d in 0..dim {
            let noise = gaussian_noise(config.seed.wrapping_add(i as u64).wrapping_add(d as u64));
            let new_val = x[d] + drift[d] * config.dt + noise_scale * noise;
            new_x.push(new_val);
            drift_sq += drift[d] * drift[d];
        }

        new_particles.push(new_x);
        drift_magnitudes.push(drift_sq.sqrt());
    }

    let num_possible_edges = num_particles * num_particles;

    SparseMeanFieldResult {
        particles: new_particles,
        drift_magnitudes,
        num_edges: total_edges,
        num_possible_edges,
    }
}

/// Simple LCG-based Gaussian noise for deterministic simulation.
fn gaussian_noise(seed: u64) -> f64 {
    let mut state = seed;
    let lcg_next = |s: &mut u64| {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    };
    lcg_next(&mut state);
    let u1 = (state & 0xFFFFFFFF) as f64 / 4294967295.0;
    lcg_next(&mut state);
    let u2 = (state & 0xFFFFFFFF) as f64 / 4294967295.0;
    let r = ((u1.max(1e-10)).ln()).sqrt() * 2.0;
    let theta = u2 * 2.0 * std::f64::consts::PI;
    r * theta.cos()
}

// ─── S150 — Fictitious Play for Sparse Graphon MFG (Nash Convergence) ─────────

/// Result of Fictitious Play policy update.
#[derive(Debug)]
pub struct FictitiousPlayResult {
    /// Softmax policy π(a|x,μ) [num_actions].
    pub policy: Vec<f64>,
    /// Policy entropy H(π) = -Σ π(a) log(π(a)).
    pub entropy: f64,
    /// Expected Q-value E_π[Q].
    pub expected_q: f64,
    /// Policy change norm ||π_new - π_old||₁ (for convergence check).
    pub policy_change: f64,
}

impl std::fmt::Display for FictitiousPlayResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FictPlay {{ entropy={:.4}, E[Q]={:.4}, Δπ={:.6}, actions={} }}",
            self.entropy, self.expected_q, self.policy_change, self.policy.len()
        )
    }
}

/// Fictitious Play with Softmax + Entropy Regularization for Nash Convergence.
///
/// Computes the softmax policy from Q-values:
/// ```math
/// π^{k+1}(a | x, μ^k) ∝ exp( Q(x,a; μ^k) / τ )
/// ```
///
/// With entropy regularization for exploration:
/// ```math
/// L = E_π[Q] - τ · H(π)
/// ```
///
/// Where H(π) = -Σ π(a) log(π(a)) is the Shannon entropy.
///
/// # Arguments
/// * `q_values` — Q-values per action [num_actions].
/// * `temperature` — Softmax temperature τ > 0 (higher = more uniform).
/// * `entropy_weight` — Entropy bonus weight for the loss term.
/// * `previous_policy` — Previous policy for computing change norm (optional).
pub fn update_policy_fictitious_play(
    q_values: &[f64],
    temperature: f64,
    entropy_weight: f64,
    previous_policy: Option<&[f64]>,
) -> FictitiousPlayResult {
    let tau = temperature.max(1e-8); // Prevent division by zero

    // 1. Scale Q by temperature: Q / τ
    let scaled: Vec<f64> = q_values.iter().map(|&q| q / tau).collect();

    // 2. Max subtraction for numerical stability
    let max_q = scaled.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let shifted: Vec<f64> = scaled.iter().map(|&v| v - max_q).collect();

    // 3. Exponentiate
    let exp_q: Vec<f64> = shifted.iter().map(|&v| v.exp()).collect();

    // 4. Sum for normalization
    let sum_exp: f64 = exp_q.iter().sum();
    let sum_exp_safe = sum_exp.max(1e-30);

    // 5. Softmax policy: π(a) = exp(Q/τ) / Σ exp(Q/τ)
    let policy: Vec<f64> = exp_q.iter().map(|&v| v / sum_exp_safe).collect();

    // 6. Entropy: H(π) = -Σ π(a) log(π(a))
    let entropy: f64 = policy.iter().map(|&p| {
        if p > 1e-15 {
            -p * p.ln()
        } else {
            0.0
        }
    }).sum();

    // 7. Expected Q: E_π[Q] = Σ π(a) · Q(a)
    let expected_q: f64 = policy.iter().zip(q_values.iter()).map(|(&pi, &q)| pi * q).sum();

    // 8. Policy change: ||π_new - π_old||₁
    let policy_change = if let Some(prev) = previous_policy {
        if prev.len() == policy.len() {
            policy.iter().zip(prev.iter()).map(|(&a, &b)| (a - b).abs()).sum()
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Entropy weight is available for loss computation but doesn't change the policy
    // (it affects the gradient in training, not the softmax itself)
    let _entropy_bonus = entropy_weight * entropy;

    FictitiousPlayResult {
        policy,
        entropy,
        expected_q,
        policy_change,
    }
}

/// Run Fictitious Play iterations until convergence or max iterations.
///
/// Simulates iterative best-response dynamics where each agent observes
/// the empirical distribution of opponent strategies and updates via softmax.
///
/// # Arguments
/// * `q_oracle` — Oracle that computes Q-values given current population policy.
/// * `temperature` — Softmax temperature.
/// * `entropy_weight` — Entropy regularization weight.
/// * `max_iters` — Maximum iterations.
/// * `convergence_tol` — Convergence threshold on policy change ||Δπ||₁.
pub fn run_fictitious_play<F>(
    q_oracle: F,
    temperature: f64,
    entropy_weight: f64,
    max_iters: usize,
    convergence_tol: f64,
) -> FictitiousPlayResult
where
    F: Fn(&[f64]) -> Vec<f64>, // population_policy -> Q_values
{
    let num_actions = q_oracle(&[]).len();
    let mut current_policy = vec![1.0 / num_actions as f64; num_actions];

    let mut last_result = FictitiousPlayResult {
        policy: current_policy.clone(),
        entropy: (num_actions as f64).ln(),
        expected_q: 0.0,
        policy_change: 0.0,
    };

    for _ in 0..max_iters {
        // Query Q-values with current population policy
        let q_values = q_oracle(&current_policy);

        // Update policy via softmax
        let result = update_policy_fictitious_play(
            &q_values,
            temperature,
            entropy_weight,
            Some(&current_policy),
        );

        // Check convergence
        if result.policy_change < convergence_tol {
            return result;
        }

        current_policy = result.policy.clone();
        last_result = result;
    }

    last_result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a particle at a given position.
    fn make_particle(values: &[f64]) -> Vec<f64> {
        values.to_vec()
    }

    /// Helper: create multiple particles.
    fn make_particles(data: &[&[f64]]) -> Vec<Vec<f64>> {
        data.iter().map(|&row| make_particle(row)).collect()
    }

    // --- Config Tests ---

    #[test]
    fn test_mean_field_config_default() {
        let cfg = MeanFieldConfig::default();
        assert_eq!(cfg.eta, 0.5);
        assert_eq!(cfg.delta, 0.3);
        assert_eq!(cfg.sigma, 0.1);
        assert_eq!(cfg.dt, 0.01);
        assert_eq!(cfg.safety_radius, 10.0);
        assert_eq!(cfg.seed, 42);
    }

    #[test]
    fn test_mean_field_config_fast() {
        let cfg = MeanFieldConfig::fast();
        assert_eq!(cfg.eta, 1.0);
        assert_eq!(cfg.sigma, 0.02);
        assert_eq!(cfg.dt, 0.05);
    }

    #[test]
    fn test_mean_field_config_exploratory() {
        let cfg = MeanFieldConfig::exploratory();
        assert_eq!(cfg.eta, 0.1);
        assert_eq!(cfg.sigma, 0.3);
        assert_eq!(cfg.dt, 0.005);
    }

    #[test]
    fn test_mean_field_config_conservative() {
        let cfg = MeanFieldConfig::conservative();
        assert_eq!(cfg.eta, 0.2);
        assert_eq!(cfg.delta, 0.8);
        assert_eq!(cfg.sigma, 0.01);
        assert_eq!(cfg.safety_radius, 5.0);
    }

    #[test]
    fn test_mean_field_config_with_eta() {
        let cfg = MeanFieldConfig::default().with_eta(0.8);
        assert_eq!(cfg.eta, 0.8);
    }

    #[test]
    fn test_mean_field_config_eta_clamped_high() {
        let cfg = MeanFieldConfig::default().with_eta(3.0);
        assert_eq!(cfg.eta, 2.0);
    }

    #[test]
    fn test_mean_field_config_eta_clamped_low() {
        let cfg = MeanFieldConfig::default().with_eta(-1.0);
        assert_eq!(cfg.eta, 0.0);
    }

    #[test]
    fn test_mean_field_config_with_delta() {
        let cfg = MeanFieldConfig::default().with_delta(0.6);
        assert_eq!(cfg.delta, 0.6);
    }

    #[test]
    fn test_mean_field_config_with_sigma() {
        let cfg = MeanFieldConfig::default().with_sigma(0.2);
        assert_eq!(cfg.sigma, 0.2);
    }

    #[test]
    fn test_mean_field_config_with_dt() {
        let cfg = MeanFieldConfig::default().with_dt(0.02);
        assert_eq!(cfg.dt, 0.02);
    }

    #[test]
    fn test_mean_field_config_dt_clamped_low() {
        let cfg = MeanFieldConfig::default().with_dt(0.0);
        assert_eq!(cfg.dt, 0.000_1);
    }

    #[test]
    fn test_mean_field_config_with_safety_radius() {
        let cfg = MeanFieldConfig::default().with_safety_radius(20.0);
        assert_eq!(cfg.safety_radius, 20.0);
    }

    #[test]
    fn test_mean_field_config_with_seed() {
        let cfg = MeanFieldConfig::default().with_seed(123);
        assert_eq!(cfg.seed, 123);
    }

    // --- Mean Field Computation Tests ---

    #[test]
    fn test_compute_mean_field_empty() {
        let result = compute_mean_field(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_compute_mean_field_single() {
        let particles = vec![make_particle(&[1.0, 2.0, 3.0])];
        let mean = compute_mean_field(&particles);
        assert!((mean[0] - 1.0).abs() < 1e-10);
        assert!((mean[1] - 2.0).abs() < 1e-10);
        assert!((mean[2] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_mean_field_uniform() {
        let particles = make_particles(&[&[0.0, 0.0], &[2.0, 2.0]]);
        let mean = compute_mean_field(&particles);
        assert!((mean[0] - 1.0).abs() < 1e-10);
        assert!((mean[1] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_mean_field_symmetric() {
        let particles = make_particles(&[&[-1.0, 1.0], &[1.0, -1.0]]);
        let mean = compute_mean_field(&particles);
        assert!(mean[0].abs() < 1e-10);
        assert!(mean[1].abs() < 1e-10);
    }

    // --- Dispersion Tests ---

    #[test]
    fn test_compute_dispersion_identical() {
        let particles = make_particles(&[&[1.0, 1.0], &[1.0, 1.0]]);
        let mean = compute_mean_field(&particles);
        let disp = compute_dispersion(&particles, &mean);
        assert!(disp < 1e-10);
    }

    #[test]
    fn test_compute_dispersion_spread() {
        let particles = make_particles(&[&[0.0, 0.0], &[2.0, 2.0]]);
        let mean = compute_mean_field(&particles);
        let disp = compute_dispersion(&particles, &mean);
        // Each particle is sqrt(2) from mean, squared = 2. Average = 2.
        assert!((disp - 2.0).abs() < 1e-10);
    }

    // --- Drift Component Tests ---

    #[test]
    fn test_vfe_drift_sign() {
        let x = vec![1.0, -2.0, 3.0];
        let drift = vfe_drift(&x);
        assert!((drift[0] - (-1.0)).abs() < 1e-10);
        assert!((drift[1] - 2.0).abs() < 1e-10);
        assert!((drift[2] - (-3.0)).abs() < 1e-10);
    }

    #[test]
    fn test_vfe_drift_zero() {
        let x = vec![0.0, 0.0, 0.0];
        let drift = vfe_drift(&x);
        for &d in &drift {
            assert!(d.abs() < 1e-10);
        }
    }

    #[test]
    fn test_mean_field_coupling_attracts() {
        let x = vec![2.0, 0.0];
        let mu = vec![0.0, 0.0];
        let coupling = mean_field_coupling(&x, &mu, 1.0);
        // Should point toward mean: (-2, 0)
        assert!((coupling[0] - (-2.0)).abs() < 1e-10);
        assert!(coupling[1].abs() < 1e-10);
    }

    #[test]
    fn test_mean_field_coupling_zero_at_mean() {
        let x = vec![1.0, 1.0];
        let mu = vec![1.0, 1.0];
        let coupling = mean_field_coupling(&x, &mu, 1.0);
        for &c in &coupling {
            assert!(c.abs() < 1e-10);
        }
    }

    #[test]
    fn test_barrier_gradient_inside() {
        let x = vec![1.0, 0.0];
        let (grad, activated) = barrier_gradient(&x, 5.0);
        assert!(!activated);
        // Gradient should point outward (repulsive barrier)
        assert!(grad[0] > 0.0);
    }

    #[test]
    fn test_barrier_gradient_outside() {
        let x = vec![6.0, 0.0];
        let (_grad, activated) = barrier_gradient(&x, 5.0);
        assert!(activated);
    }

    #[test]
    fn test_barrier_gradient_at_origin() {
        let x = vec![0.0, 0.0, 0.0];
        let (grad, activated) = barrier_gradient(&x, 5.0);
        assert!(!activated);
        for &g in &grad {
            assert!(g.abs() < 1e-10);
        }
    }

    // --- Full Drift Tests ---

    #[test]
    fn test_compute_drift_shape() {
        let x = vec![1.0, 2.0, 3.0];
        let mu = vec![0.0, 0.0, 0.0];
        let cfg = MeanFieldConfig::default();
        let (drift, _activated) = compute_drift(&x, &mu, &cfg);
        assert_eq!(drift.len(), 3);
    }

    #[test]
    fn test_compute_drift_pushes_toward_origin() {
        // Particle far from origin + mean at origin should drift inward
        let x = vec![5.0, 0.0];
        let mu = vec![0.0, 0.0];
        let cfg = MeanFieldConfig::default();
        let (drift, _activated) = compute_drift(&x, &mu, &cfg);
        // VFE drift = -5, coupling = -5 (toward mean=0), barrier pushes outward
        // Net should be negative (inward) for first component
        assert!(drift[0] < 0.0);
    }

    // --- Single Step Tests ---

    #[test]
    fn test_mean_field_step_empty() {
        let cfg = MeanFieldConfig::default();
        let result = mean_field_step(&[], &cfg);
        assert!(result.particles.is_empty());
        assert!(result.mean_field.is_empty());
        assert_eq!(result.dispersion, 0.0);
    }

    #[test]
    fn test_mean_field_step_preserves_count() {
        let particles = make_particles(&[&[1.0, 0.0], &[0.0, 1.0], &[-1.0, 0.0]]);
        let cfg = MeanFieldConfig::default();
        let result = mean_field_step(&particles, &cfg);
        assert_eq!(result.particles.len(), 3);
    }

    #[test]
    fn test_mean_field_step_preserves_dim() {
        let particles = make_particles(&[&[1.0, 2.0, 3.0, 4.0]]);
        let cfg = MeanFieldConfig::default();
        let result = mean_field_step(&particles, &cfg);
        assert_eq!(result.particles[0].len(), 4);
    }

    #[test]
    fn test_mean_field_step_reduces_dispersion() {
        // Particles spread apart should converge toward mean with strong coupling
        let particles = make_particles(&[&[-5.0, 0.0], &[5.0, 0.0]]);
        let cfg = MeanFieldConfig {
            eta: 2.0, // Strong coupling
            sigma: 0.0, // No noise
            dt: 0.1,
            ..MeanFieldConfig::default()
        };
        let result = mean_field_step(&particles, &cfg);
        let initial_disp = compute_dispersion(&particles, &compute_mean_field(&particles));
        let final_disp = result.dispersion;
        assert!(final_disp < initial_disp, "Dispersion should decrease with strong coupling and no noise");
    }

    #[test]
    fn test_mean_field_step_deterministic_with_seed() {
        let particles = make_particles(&[&[1.0, 0.0], &[0.0, 1.0]]);
        let cfg = MeanFieldConfig::default();
        let r1 = mean_field_step(&particles, &cfg);
        let r2 = mean_field_step(&particles, &cfg);
        assert_eq!(r1.particles.len(), r2.particles.len());
        for (p1, p2) in r1.particles.iter().zip(r2.particles.iter()) {
            for (v1, v2) in p1.iter().zip(p2.iter()) {
                assert!((v1 - v2).abs() < 1e-10, "Same seed should produce same result");
            }
        }
    }

    #[test]
    fn test_mean_field_step_barrier_activation() {
        // Particle near safety boundary should trigger barrier
        let particles = make_particles(&[&[9.9, 0.0]]);
        let cfg = MeanFieldConfig {
            safety_radius: 10.0,
            delta: 1.0,
            ..MeanFieldConfig::default()
        };
        let result = mean_field_step(&particles, &cfg);
        // With high barrier weight, particle should be pushed inward
        let norm_sq: f64 = result.particles[0].iter().map(|v| v * v).sum();
        assert!(norm_sq < 10.0 * 10.0 + 1.0, "Barrier should prevent escape");
    }

    // --- Trajectory Tests ---

    #[test]
    fn test_mean_field_trajectory_length() {
        let particles = make_particles(&[&[1.0, 0.0], &[0.0, 1.0]]);
        let cfg = MeanFieldConfig::default();
        let traj = mean_field_trajectory(&particles, &cfg, 10);
        assert_eq!(traj.len(), 10);
    }

    #[test]
    fn test_mean_field_trajectory_convergence() {
        // With strong coupling and no noise, particles should converge
        let particles = make_particles(&[&[-2.0, 0.0], &[2.0, 0.0]]);
        let cfg = MeanFieldConfig {
            eta: 2.0,
            sigma: 0.0,
            dt: 0.1,
            ..MeanFieldConfig::default()
        };
        let traj = mean_field_trajectory(&particles, &cfg, 20);
        let final_disp = traj.last().unwrap().dispersion;
        assert!(final_disp < 0.5, "Particles should converge with strong coupling");
    }

    #[test]
    fn test_mean_field_trajectory_dispersion_trend() {
        let particles = make_particles(&[&[-1.0, 0.0], &[1.0, 0.0]]);
        let cfg = MeanFieldConfig {
            eta: 1.0,
            sigma: 0.0,
            dt: 0.05,
            ..MeanFieldConfig::default()
        };
        let traj = mean_field_trajectory(&particles, &cfg, 15);
        // Dispersion should generally decrease
        let first_disp = traj.first().unwrap().dispersion;
        let last_disp = traj.last().unwrap().dispersion;
        assert!(last_disp < first_disp, "Dispersion should decrease over time");
    }

    // --- Safety Invariant Tests ---

    #[test]
    fn test_verify_safety_invariant_all_safe() {
        let particles = make_particles(&[&[1.0, 0.0], &[0.0, 1.0]]);
        let cfg = MeanFieldConfig {
            safety_radius: 10.0,
            ..MeanFieldConfig::default()
        };
        let (safe, violations) = verify_safety_invariant(&particles, &cfg);
        assert!(safe);
        assert_eq!(violations, 0);
    }

    #[test]
    fn test_verify_safety_invariant_violation() {
        let particles = make_particles(&[&[1.0, 0.0], &[15.0, 0.0]]);
        let cfg = MeanFieldConfig {
            safety_radius: 10.0,
            ..MeanFieldConfig::default()
        };
        let (safe, violations) = verify_safety_invariant(&particles, &cfg);
        assert!(!safe);
        assert_eq!(violations, 1);
    }

    #[test]
    fn test_verify_safety_invariant_boundary() {
        let particles = make_particles(&[&[10.0, 0.0]]);
        let cfg = MeanFieldConfig {
            safety_radius: 10.0,
            ..MeanFieldConfig::default()
        };
        let (safe, violations) = verify_safety_invariant(&particles, &cfg);
        // Exactly on boundary — norm_sq == r_sq, so not a violation
        assert!(safe);
        assert_eq!(violations, 0);
    }

    // --- Display Tests ---

    #[test]
    fn test_mean_field_step_result_display() {
        let particles = make_particles(&[&[1.0, 0.0]]);
        let cfg = MeanFieldConfig::default();
        let result = mean_field_step(&particles, &cfg);
        let display = format!("{}", result);
        assert!(display.contains("MeanFieldStep"));
        assert!(display.contains("particles: 1"));
    }

    // --- Integration: Full Symbiotic Mean-Field Pipeline ---

    #[test]
    fn test_full_mean_field_pipeline() {
        // Initialize 4 particles in 2D
        let particles = make_particles(&[
            &[1.0, 0.0],
            &[-1.0, 0.0],
            &[0.0, 1.0],
            &[0.0, -1.0],
        ]);
        let cfg = MeanFieldConfig {
            eta: 0.8,
            delta: 0.3,
            sigma: 0.05,
            dt: 0.02,
            safety_radius: 10.0,
            seed: 42,
        };

        // Run trajectory
        let traj = mean_field_trajectory(&particles, &cfg, 50);
        assert_eq!(traj.len(), 50);

        // Verify particle count preserved
        for step in &traj {
            assert_eq!(step.particles.len(), 4);
            assert_eq!(step.particles[0].len(), 2);
        }

        // Verify dispersion decreases (convergence)
        let initial_disp = traj.first().unwrap().dispersion;
        let final_disp = traj.last().unwrap().dispersion;
        assert!(
            final_disp < initial_disp,
            "Final dispersion ({:.4}) should be less than initial ({:.4})",
            final_disp,
            initial_disp
        );

        // Verify safety invariant at final step
        let (safe, violations) =
            verify_safety_invariant(&traj.last().unwrap().particles, &cfg);
        assert!(
            safe,
            "All particles should remain within safety boundary (violations: {})",
            violations
        );

        // Verify mean field is computed correctly
        let final_mean = &traj.last().unwrap().mean_field;
        assert_eq!(final_mean.len(), 2);
    }

    /// Sprint 145 summary test — validates McKean-Vlasov SDE implementation.
    #[test]
    fn test_s145_mean_field_summary() {
        let cfg = MeanFieldConfig::default();
        assert!(cfg.eta > 0.0, "Coupling must be positive");
        assert!(cfg.delta > 0.0, "Barrier must be active");
        assert!(cfg.sigma >= 0.0, "Diffusion must be non-negative");
        assert!(cfg.dt > 0.0, "Time step must be positive");

        let particles = make_particles(&[&[1.0, 0.0], &[0.0, 1.0]]);
        let result = mean_field_step(&particles, &cfg);
        assert_eq!(result.particles.len(), 2, "Particle count preserved");
        assert_eq!(result.mean_field.len(), 2, "Mean field dimension correct");
        assert!(result.dispersion >= 0.0, "Dispersion non-negative");
    }
}
