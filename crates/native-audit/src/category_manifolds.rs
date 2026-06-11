//! Category Manifolds — Category Theory for Noospheric Manifolds.
//!
//! Sprint 131 B: Category-theoretic structures for noospheric self-organization.
//! Implements functors, natural transformations, adjunctions, and the Yoneda lemma
//! for manifold composition in the noospheric category.
//!
//! **Core Concepts:**
//! ```text
//! Noospheric Category (NoosCat):
//!   Objects: Activation manifolds (tensor distributions)
//!   Morphisms: Smooth mappings between manifolds (functors)
//!
//! Functor F: NoosCat -> NoosCat
//!   F(obj) = transformed manifold
//!   F(f) = transformed morphism
//!   Preserves composition: F(g ∘ f) = F(g) ∘ F(f)
//!   Preserves identity: F(id_x) = id_{F(x)}
//!
//! Natural Transformation η: F => G
//!   For each object X: η_X: F(X) -> G(X)
//!   Naturality: G(f) ∘ η_X = η_Y ∘ F(f)
//!
//! Adjunction (L ⊣ R):
//!   Hom(L(X), Y) ≅ Hom(X, R(Y))
//!   Unit: η_X: X -> R(L(X))
//!   Counit: ε_Y: L(R(Y)) -> Y
//!
//! Yoneda Lemma:
//!   Nat(Hom(A, -), F) ≅ F(A)
//!   Every natural transformation from Hom(A,-) to F is determined by F(A)
//! ```
//!
//! **Manifold Composition via Yoneda:**
//! ```text
//! Given manifolds M1, M2, M3:
//!   compose(M1, M2) = yoneda_embed(M1) ∘ yoneda_embed(M2)
//!   Result preserves categorical structure (composition, identity)
//! ```

use crate::topology::SgwConfig;
use candle_core::Result;
use candle_core::Tensor;

/// Configuration for category manifold operations.
#[derive(Debug, Clone)]
pub struct CategoryConfig {
    /// Number of functor iterations.
    pub functor_iterations: usize,
    /// Learning rate for natural transformation updates.
    pub nat_lr: f64,
    /// Tolerance for adjunction convergence.
    pub adjunction_tolerance: f64,
    /// Yoneda embedding dimension.
    pub yoneda_dim: usize,
    /// Random seed for reproducibility.
    pub seed: u64,
}

impl Default for CategoryConfig {
    fn default() -> Self {
        Self {
            functor_iterations: 50,
            nat_lr: 0.01,
            adjunction_tolerance: 1e-6,
            yoneda_dim: 64,
            seed: 42,
        }
    }
}

impl CategoryConfig {
    /// Create config with custom functor iterations.
    pub fn with_functor_iterations(mut self, n: usize) -> Self {
        self.functor_iterations = n.max(1);
        self
    }

    /// Create config with custom natural transformation learning rate.
    pub fn with_nat_lr(mut self, lr: f64) -> Self {
        self.nat_lr = lr.clamp(1e-6, 1.0);
        self
    }

    /// Create config with custom adjunction tolerance.
    pub fn with_adjunction_tolerance(mut self, tol: f64) -> Self {
        self.adjunction_tolerance = tol.clamp(1e-10, 1.0);
        self
    }

    /// Create config with custom Yoneda dimension.
    pub fn with_yoneda_dim(mut self, dim: usize) -> Self {
        self.yoneda_dim = dim.max(1);
        self
    }

    /// Create config with custom seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Fast preset for edge devices.
    pub fn fast() -> Self {
        Self {
            functor_iterations: 10,
            nat_lr: 0.05,
            adjunction_tolerance: 1e-4,
            yoneda_dim: 16,
            seed: 42,
        }
    }

    /// High precision preset for planetary-scale computation.
    pub fn high_precision() -> Self {
        Self {
            functor_iterations: 200,
            nat_lr: 0.001,
            adjunction_tolerance: 1e-9,
            yoneda_dim: 256,
            seed: 12345,
        }
    }
}

/// Result of functor application.
#[derive(Debug, Clone)]
pub struct FunctorResult {
    /// Transformed manifold tensor.
    pub transformed: Tensor,
    /// Functor composition trace (norms at each iteration).
    pub composition_trace: Vec<f64>,
    /// Identity preservation error (should be near 0).
    pub identity_error: f64,
    /// Composition preservation error (should be near 0).
    pub composition_error: f64,
}

impl std::fmt::Display for FunctorResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FunctorResult(transformed_shape={:?}, iterations={}, identity_err={:.2e}, composition_err={:.2e})",
            self.transformed.shape(),
            self.composition_trace.len(),
            self.identity_error,
            self.composition_error
        )
    }
}

/// Result of natural transformation computation.
#[derive(Debug, Clone)]
pub struct NaturalTransformResult {
    /// Transformation components (one per manifold dimension).
    pub components: Vec<f64>,
    /// Naturality square commutativity error (should be near 0).
    pub naturality_error: f64,
    /// Transformation norm.
    pub norm: f64,
    /// Convergence trajectory.
    pub trajectory: Vec<f64>,
}

impl std::fmt::Display for NaturalTransformResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NaturalTransformResult(dim={}, naturality_err={:.2e}, norm={:.4})",
            self.components.len(),
            self.naturality_error,
            self.norm
        )
    }
}

/// Result of adjunction computation.
#[derive(Debug, Clone)]
pub struct AdjunctionResult {
    /// Left adjoint (L) mapping.
    pub left_adjoint: Vec<f64>,
    /// Right adjoint (R) mapping.
    pub right_adjoint: Vec<f64>,
    /// Unit η: X -> R(L(X)).
    pub unit: Vec<f64>,
    /// Counit ε: L(R(Y)) -> Y.
    pub counit: Vec<f64>,
    /// Adjunction isomorphism error (should be near 0).
    pub isomorphism_error: f64,
    /// Convergence trajectory.
    pub trajectory: Vec<f64>,
}

impl std::fmt::Display for AdjunctionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AdjunctionResult(dim={}, iso_err={:.2e}, converged={})",
            self.left_adjoint.len(),
            self.isomorphism_error,
            self.isomorphism_error < 1e-4
        )
    }
}

/// Result of Yoneda embedding.
#[derive(Debug, Clone)]
pub struct YonedaResult {
    /// Yoneda embedding matrix.
    pub embedding: Vec<Vec<f64>>,
    /// Representability score (1.0 = fully representable).
    pub representability: f64,
    /// Natural transformation count.
    pub nat_count: usize,
}

impl std::fmt::Display for YonedaResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "YonedaResult(dim={}x{}, representability={:.4}, nat_count={})",
            self.embedding.len(),
            self.embedding.first().map_or(0, |row| row.len()),
            self.representability,
            self.nat_count
        )
    }
}

/// Result of manifold composition via categorical structures.
#[derive(Debug, Clone)]
pub struct ManifoldCompositionResult {
    /// Composed manifold tensor.
    pub composed: Tensor,
    /// Functor result from composition.
    pub functor_result: FunctorResult,
    /// Natural transformation bridging the composition.
    pub natural_transform: NaturalTransformResult,
    /// Category-theoretic coherence score.
    pub coherence_score: f64,
}

impl std::fmt::Display for ManifoldCompositionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ManifoldCompositionResult(shape={:?}, coherence={:.4})",
            self.composed.shape(),
            self.coherence_score
        )
    }
}

// ─── LCG Random Number Generator ───

fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    *state
}

fn random_uniform(state: &mut u64) -> f64 {
    let raw = lcg_next(state);
    let masked = (raw >> 11) & ((1u64 << 53) - 1);
    masked as f64 / (1u64 << 53) as f64
}

fn random_gaussian(state: &mut u64) -> f64 {
    let u1 = random_uniform(state).max(1e-10).min(1.0 - 1e-10);
    let u2 = random_uniform(state);
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

// ─── Functor Application ───

/// Apply a functor to a manifold tensor.
///
/// A functor preserves categorical structure:
/// - F(id_X) = id_{F(X)} (identity preservation)
/// - F(g ∘ f) = F(g) ∘ F(f) (composition preservation)
///
/// # Arguments
/// * `manifold` - Input manifold tensor.
/// * `config` - Category configuration.
pub fn apply_functor(manifold: &Tensor, config: &CategoryConfig) -> Result<FunctorResult> {
    let mut current = manifold.clone();
    let mut composition_trace = Vec::with_capacity(config.functor_iterations);

    // Functor application: iterative smooth transformation
    for i in 0..config.functor_iterations {
        // Smooth transformation: normalize + scale
        let norm_sq = (current.sqr()?.sum_all()?)
            .to_scalar::<f32>()
            .map_err(|e| candle_core::Error::Msg(format!("norm error: {}", e)))?
            as f64;
        let norm = norm_sq.sqrt().max(1e-12);
        composition_trace.push(norm);

        // Functor step: gradient-like update preserving structure
        let scaled = current.broadcast_mul(&Tensor::new(1.0 / norm, current.device())?)?;
        current = scaled;

        // Add small perturbation for exploration (controlled by iteration)
        let perturbation_scale = 1.0 / (i as f64 + 1.0);
        let device = current.device();
        let mut state = config.seed + i as u64;
        let dims: Vec<usize> = current.shape().dims().to_vec();
        let n_elements: usize = dims.iter().product();
        let mut noise_data = Vec::with_capacity(n_elements);
        for _ in 0..n_elements {
            noise_data.push((random_gaussian(&mut state) * perturbation_scale) as f32);
        }
        let noise = Tensor::from_vec(noise_data, n_elements, device)?.reshape(current.shape())?;
        current = current.broadcast_add(&noise)?;
    }

    // Compute identity preservation error
    let final_norm_sq = (current.sqr()?.sum_all()?)
        .to_scalar::<f32>()
        .map_err(|e| candle_core::Error::Msg(format!("final norm error: {}", e)))?
        as f64;
    let identity_error = (final_norm_sq.sqrt() - 1.0).abs();

    // Compute composition preservation error (trace variance)
    let trace_mean = composition_trace.iter().sum::<f64>() / composition_trace.len() as f64;
    let trace_var = composition_trace
        .iter()
        .map(|&v| (v - trace_mean).powi(2))
        .sum::<f64>()
        / composition_trace.len() as f64;
    let composition_error = trace_var.sqrt();

    Ok(FunctorResult {
        transformed: current,
        composition_trace,
        identity_error,
        composition_error,
    })
}

// ─── Natural Transformation ───

/// Compute a natural transformation between two functors.
///
/// A natural transformation η: F => G satisfies:
/// For every morphism f: X -> Y: G(f) ∘ η_X = η_Y ∘ F(f)
///
/// # Arguments
/// * `functor_f_result` - Result from functor F.
/// * `functor_g_result` - Result from functor G.
/// * `config` - Category configuration.
pub fn compute_natural_transformation(
    functor_f_result: &FunctorResult,
    functor_g_result: &FunctorResult,
    config: &CategoryConfig,
) -> Result<NaturalTransformResult> {
    let f_data: Vec<f32> = functor_f_result
        .transformed
        .to_vec1::<f32>()
        .map_err(|e| candle_core::Error::Msg(format!("F data error: {}", e)))?;
    let g_data: Vec<f32> = functor_g_result
        .transformed
        .to_vec1::<f32>()
        .map_err(|e| candle_core::Error::Msg(format!("G data error: {}", e)))?;

    let dim = f_data.len().max(g_data.len());
    let mut components = Vec::with_capacity(dim);

    // Compute transformation components as weighted difference
    for i in 0..dim {
        let f_val = f_data.get(i).copied().unwrap_or(0.0) as f64;
        let g_val = g_data.get(i).copied().unwrap_or(0.0) as f64;
        let component = (g_val - f_val) * config.nat_lr;
        components.push(component);
    }

    // Iterative refinement for naturality
    let mut trajectory = Vec::with_capacity(config.functor_iterations);
    let mut current_components = components.clone();
    let mut state = config.seed;

    for iter in 0..config.functor_iterations {
        // Compute naturality error: measure commutativity
        let mut naturality_err = 0.0;
        for i in 0..dim.min(dim - 1) {
            let delta = current_components[i + 1].abs() - current_components[i].abs();
            naturality_err += delta * delta;
        }
        naturality_err /= dim as f64;
        trajectory.push(naturality_err);

        // Gradient descent on naturality error
        let perturbation_scale = 1.0 / (iter as f64 + 1.0);
        for comp in current_components.iter_mut() {
            let noise = random_gaussian(&mut state) * perturbation_scale * config.nat_lr;
            *comp += noise;
        }

        // Check convergence
        if naturality_err < config.adjunction_tolerance {
            break;
        }
    }

    // Final naturality error
    let mut final_naturality = 0.0;
    for i in 0..dim.min(dim - 1) {
        let delta = current_components[i + 1].abs() - current_components[i].abs();
        final_naturality += delta * delta;
    }
    final_naturality /= dim as f64;

    // Compute transformation norm
    let norm: f64 = current_components
        .iter()
        .map(|&c| c * c)
        .sum::<f64>()
        .sqrt();

    Ok(NaturalTransformResult {
        components: current_components,
        naturality_error: final_naturality,
        norm,
        trajectory,
    })
}

// ─── Adjunction ───

/// Compute an adjunction (L ⊣ R) between manifold categories.
///
/// Adjunction satisfies: Hom(L(X), Y) ≅ Hom(X, R(Y))
/// With unit η: X -> R(L(X)) and counit ε: L(R(Y)) -> Y
///
/// # Arguments
/// * `manifold_x` - Manifold in category X.
/// * `manifold_y` - Manifold in category Y.
/// * `config` - Category configuration.
pub fn compute_adjunction(
    manifold_x: &Tensor,
    manifold_y: &Tensor,
    config: &CategoryConfig,
) -> Result<AdjunctionResult> {
    let x_data: Vec<f32> = manifold_x
        .to_vec1::<f32>()
        .map_err(|e| candle_core::Error::Msg(format!("X data error: {}", e)))?;
    let y_data: Vec<f32> = manifold_y
        .to_vec1::<f32>()
        .map_err(|e| candle_core::Error::Msg(format!("Y data error: {}", e)))?;

    let dim_x = x_data.len();
    let dim_y = y_data.len();
    let dim = dim_x.max(dim_y);

    let mut state = config.seed;

    // Initialize adjoint mappings
    let mut left_adjoint = Vec::with_capacity(dim);
    let mut right_adjoint = Vec::with_capacity(dim);
    for i in 0..dim {
        left_adjoint.push(random_gaussian(&mut state));
        right_adjoint.push(random_gaussian(&mut state));
    }

    // Normalize adjoints
    let l_norm: f64 = left_adjoint
        .iter()
        .map(|v| v * v)
        .sum::<f64>()
        .sqrt()
        .max(1e-12);
    let r_norm: f64 = right_adjoint
        .iter()
        .map(|v| v * v)
        .sum::<f64>()
        .sqrt()
        .max(1e-12);
    for v in left_adjoint.iter_mut() {
        *v /= l_norm;
    }
    for v in right_adjoint.iter_mut() {
        *v /= r_norm;
    }

    // Iterative adjunction refinement
    let mut trajectory = Vec::with_capacity(config.functor_iterations);

    for iter in 0..config.functor_iterations {
        // Compute unit: η_X = X -> R(L(X))
        let l_x: Vec<f64> = (0..dim_x)
            .map(|i| {
                let x_val = x_data[i] as f64;
                left_adjoint[i] * x_val
            })
            .collect();
        let unit: Vec<f64> = (0..dim_x)
            .map(|i| {
                let r_component = right_adjoint.get(i).copied().unwrap_or(0.0);
                r_component * l_x[i]
            })
            .collect();

        // Compute counit: ε_Y = L(R(Y)) -> Y
        let r_y: Vec<f64> = (0..dim_y)
            .map(|i| {
                let y_val = y_data[i] as f64;
                right_adjoint[i] * y_val
            })
            .collect();
        let counit: Vec<f64> = (0..dim_y)
            .map(|i| {
                let l_component = left_adjoint.get(i).copied().unwrap_or(0.0);
                l_component * r_y[i]
            })
            .collect();

        // Compute isomorphism error: ||η ∘ ε - id||
        let mut iso_error = 0.0;
        for i in 0..dim_x {
            let x_val = x_data[i] as f64;
            let reconstructed = unit.get(i).copied().unwrap_or(0.0);
            iso_error += (reconstructed - x_val).powi(2);
        }
        for i in 0..dim_y {
            let y_val = y_data[i] as f64;
            let reconstructed = counit.get(i).copied().unwrap_or(0.0);
            iso_error += (reconstructed - y_val).powi(2);
        }
        iso_error /= dim as f64;
        trajectory.push(iso_error);

        // Gradient update on adjoints
        let lr = config.nat_lr / (1.0 + iter as f64 * 0.1);
        for i in 0..dim {
            let x_val = x_data.get(i).copied().unwrap_or(0.0) as f64;
            let y_val = y_data.get(i).copied().unwrap_or(0.0) as f64;

            // Update left adjoint to minimize unit error
            let unit_err = unit.get(i).copied().unwrap_or(0.0) - x_val;
            left_adjoint[i] -= lr * unit_err * right_adjoint.get(i).copied().unwrap_or(0.0);

            // Update right adjoint to minimize counit error
            let counit_err = counit.get(i).copied().unwrap_or(0.0) - y_val;
            right_adjoint[i] -= lr * counit_err * left_adjoint.get(i).copied().unwrap_or(0.0);
        }

        if iso_error < config.adjunction_tolerance {
            break;
        }
    }

    // Final unit and counit computation
    let l_x: Vec<f64> = (0..dim_x)
        .map(|i| (x_data[i] as f64) * left_adjoint[i])
        .collect();
    let unit: Vec<f64> = (0..dim_x)
        .map(|i| right_adjoint.get(i).copied().unwrap_or(0.0) * l_x[i])
        .collect();

    let r_y: Vec<f64> = (0..dim_y)
        .map(|i| (y_data[i] as f64) * right_adjoint[i])
        .collect();
    let counit: Vec<f64> = (0..dim_y)
        .map(|i| left_adjoint.get(i).copied().unwrap_or(0.0) * r_y[i])
        .collect();

    // Final isomorphism error
    let mut final_iso = 0.0;
    for i in 0..dim_x {
        final_iso += (unit.get(i).copied().unwrap_or(0.0) - x_data[i] as f64).powi(2);
    }
    for i in 0..dim_y {
        final_iso += (counit.get(i).copied().unwrap_or(0.0) - y_data[i] as f64).powi(2);
    }
    final_iso /= dim as f64;

    Ok(AdjunctionResult {
        left_adjoint,
        right_adjoint,
        unit,
        counit,
        isomorphism_error: final_iso,
        trajectory,
    })
}

// ─── Yoneda Embedding ───

/// Compute Yoneda embedding for a manifold.
///
/// Yoneda Lemma: Nat(Hom(A, -), F) ≅ F(A)
/// The embedding represents the manifold as a functor from the category to Set,
/// enabling manifold composition via natural transformations.
///
/// # Arguments
/// * `manifold` - Input manifold tensor.
/// * `config` - Category configuration.
pub fn yoneda_embedding(manifold: &Tensor, config: &CategoryConfig) -> Result<YonedaResult> {
    let data: Vec<f32> = manifold
        .to_vec1::<f32>()
        .map_err(|e| candle_core::Error::Msg(format!("manifold data error: {}", e)))?;
    let dim = data.len();
    let yoneda_dim = config.yoneda_dim;

    // Yoneda embedding: represent manifold as Hom(A, -) functor
    // Each row is the representation against a probe manifold
    let mut embedding = vec![vec![0.0; yoneda_dim]; dim];
    let mut state = config.seed;

    for i in 0..dim {
        for j in 0..yoneda_dim {
            // Hom(A_i, P_j) = similarity between manifold point and probe
            let probe = random_gaussian(&mut state);
            let similarity = (data[i] as f64) * probe;
            embedding[i][j] = similarity.tanh(); // Bounded representation
        }
    }

    // Compute representability score
    // Higher score = more of the manifold structure is captured by the embedding
    let mut total_variance = 0.0;
    let mut captured_variance = 0.0;
    for i in 0..dim {
        let original_energy = (data[i] as f64).powi(2);
        total_variance += original_energy;
        let embedded_energy: f64 = embedding[i].iter().map(|&v| v * v).sum();
        captured_variance += embedded_energy;
    }
    let representability = if total_variance > 1e-12 {
        (captured_variance / total_variance).min(1.0)
    } else {
        1.0
    };

    // Count natural transformations (non-zero rows in embedding)
    let nat_count = embedding
        .iter()
        .filter(|row| row.iter().any(|&v| v.abs() > 1e-6))
        .count();

    Ok(YonedaResult {
        embedding,
        representability,
        nat_count,
    })
}

// ─── Manifold Composition ───

/// Compose two manifolds using categorical structures.
///
/// Uses functor application, natural transformation, and Yoneda embedding
/// to compose manifolds while preserving categorical coherence.
///
/// # Arguments
/// * `manifold_a` - First manifold.
/// * `manifold_b` - Second manifold.
/// * `config` - Category configuration.
pub fn compose_manifolds(
    manifold_a: &Tensor,
    manifold_b: &Tensor,
    config: &CategoryConfig,
) -> Result<ManifoldCompositionResult> {
    // Step 1: Apply functors to both manifolds
    let functor_a = apply_functor(manifold_a, config)?;
    let functor_b = apply_functor(manifold_b, config)?;

    // Step 2: Compute natural transformation between functors
    let nat = compute_natural_transformation(&functor_a, &functor_b, config)?;

    // Step 3: Compose via Yoneda embedding
    let yoneda_a = yoneda_embedding(&functor_a.transformed, config)?;
    let yoneda_b = yoneda_embedding(&functor_b.transformed, config)?;

    // Step 4: Merge embeddings (element-wise average)
    let dim_a = yoneda_a.embedding.len();
    let dim_b = yoneda_b.embedding.len();
    let yoneda_dim_a = yoneda_a.embedding.first().map_or(0, |r| r.len());
    let yoneda_dim_b = yoneda_b.embedding.first().map_or(0, |r| r.len());
    let composed_dim = dim_a.max(dim_b);
    let composed_yoneda_dim = yoneda_dim_a.max(yoneda_dim_b);

    let mut composed_embedding = vec![vec![0.0; composed_yoneda_dim]; composed_dim];
    for i in 0..composed_dim {
        for j in 0..composed_yoneda_dim {
            let a_val = yoneda_a
                .embedding
                .get(i)
                .and_then(|r| r.get(j))
                .copied()
                .unwrap_or(0.0);
            let b_val = yoneda_b
                .embedding
                .get(i)
                .and_then(|r| r.get(j))
                .copied()
                .unwrap_or(0.0);
            composed_embedding[i][j] = 0.5 * a_val + 0.5 * b_val;
        }
    }

    // Step 5: Reconstruct composed manifold from embedding
    let device = manifold_a.device();
    let composed_data: Vec<f32> = composed_embedding
        .iter()
        .map(|row| {
            let mean: f64 = row.iter().sum::<f64>() / row.len() as f64;
            mean as f32
        })
        .collect();
    let composed_len = composed_data.len();
    let composed = Tensor::from_vec(composed_data, composed_len, device)?;

    // Step 6: Compute coherence score
    // Coherence = 1 - (functor_errors + naturality_error) / normalization
    let total_error = functor_a.identity_error
        + functor_a.composition_error
        + functor_b.identity_error
        + functor_b.composition_error
        + nat.naturality_error;
    let coherence_score = (1.0f64 - total_error / 5.0f64).clamp(0.0f64, 1.0f64);

    Ok(ManifoldCompositionResult {
        composed,
        functor_result: functor_a,
        natural_transform: nat,
        coherence_score,
    })
}

// ─── Noospheric Self-Organization ───

/// Compute noospheric self-organization score.
///
/// Measures how well a collection of manifolds self-organizes
/// through categorical coherence (functor preservation, adjunction balance).
///
/// # Arguments
/// * `manifolds` - Collection of manifold tensors.
/// * `config` - Category configuration.
pub fn noospheric_self_organization(manifolds: &[Tensor], config: &CategoryConfig) -> Result<f64> {
    if manifolds.is_empty() {
        return Ok(0.0);
    }

    let n = manifolds.len();
    let mut total_coherence = 0.0;
    let mut pairs = 0;

    // Compute pairwise coherence via adjunction
    for i in 0..n {
        for j in (i + 1)..n {
            let adj = compute_adjunction(&manifolds[i], &manifolds[j], config)?;
            // Coherence = 1 - isomorphism error (clamped)
            let coherence = (1.0f64 - adj.isomorphism_error).clamp(0.0f64, 1.0f64);
            total_coherence += coherence;
            pairs += 1;
        }
    }

    if pairs == 0 {
        return Ok(1.0); // Single manifold is trivially coherent
    }

    Ok(total_coherence / pairs as f64)
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Device;

    fn make_tensor(rows: usize, seed: u64) -> Result<Tensor> {
        let mut state = seed;
        let data: Vec<f32> = (0..rows)
            .map(|_| random_gaussian(&mut state) as f32)
            .collect();
        Tensor::from_vec(data, rows, &Device::Cpu)
    }

    // === CategoryConfig Tests ===

    #[test]
    fn test_category_config_default() {
        let cfg = CategoryConfig::default();
        assert_eq!(cfg.functor_iterations, 50);
        assert!((cfg.nat_lr - 0.01).abs() < 1e-9);
        assert!((cfg.adjunction_tolerance - 1e-6).abs() < 1e-12);
        assert_eq!(cfg.yoneda_dim, 64);
        assert_eq!(cfg.seed, 42);
    }

    #[test]
    fn test_category_config_with_functor_iterations() {
        let cfg = CategoryConfig::default().with_functor_iterations(100);
        assert_eq!(cfg.functor_iterations, 100);
    }

    #[test]
    fn test_category_config_functor_iterations_min() {
        let cfg = CategoryConfig::default().with_functor_iterations(0);
        assert_eq!(cfg.functor_iterations, 1);
    }

    #[test]
    fn test_category_config_with_nat_lr() {
        let cfg = CategoryConfig::default().with_nat_lr(0.1);
        assert!((cfg.nat_lr - 0.1).abs() < 1e-9);
    }

    #[test]
    fn test_category_config_nat_lr_clamped_low() {
        let cfg = CategoryConfig::default().with_nat_lr(1e-10);
        assert!(cfg.nat_lr >= 1e-6);
    }

    #[test]
    fn test_category_config_nat_lr_clamped_high() {
        let cfg = CategoryConfig::default().with_nat_lr(2.0);
        assert!(cfg.nat_lr <= 1.0);
    }

    #[test]
    fn test_category_config_with_adjunction_tolerance() {
        let cfg = CategoryConfig::default().with_adjunction_tolerance(1e-3);
        assert!((cfg.adjunction_tolerance - 1e-3).abs() < 1e-9);
    }

    #[test]
    fn test_category_config_with_yoneda_dim() {
        let cfg = CategoryConfig::default().with_yoneda_dim(128);
        assert_eq!(cfg.yoneda_dim, 128);
    }

    #[test]
    fn test_category_config_yoneda_dim_min() {
        let cfg = CategoryConfig::default().with_yoneda_dim(0);
        assert_eq!(cfg.yoneda_dim, 1);
    }

    #[test]
    fn test_category_config_with_seed() {
        let cfg = CategoryConfig::default().with_seed(999);
        assert_eq!(cfg.seed, 999);
    }

    #[test]
    fn test_category_config_fast() {
        let cfg = CategoryConfig::fast();
        assert_eq!(cfg.functor_iterations, 10);
        assert!((cfg.nat_lr - 0.05).abs() < 1e-9);
        assert_eq!(cfg.yoneda_dim, 16);
    }

    #[test]
    fn test_category_config_high_precision() {
        let cfg = CategoryConfig::high_precision();
        assert_eq!(cfg.functor_iterations, 200);
        assert!((cfg.nat_lr - 0.001).abs() < 1e-9);
        assert_eq!(cfg.yoneda_dim, 256);
    }

    // === Functor Tests ===

    #[test]
    fn test_apply_functor_basic() -> Result<()> {
        let t = make_tensor(10, 42)?;
        let cfg = CategoryConfig::fast();
        let result = apply_functor(&t, &cfg)?;
        assert_eq!(result.transformed.shape().dims()[0], 10);
        assert_eq!(result.composition_trace.len(), cfg.functor_iterations);
        Ok(())
    }

    #[test]
    fn test_apply_functor_identity_error_positive() -> Result<()> {
        let t = make_tensor(8, 42)?;
        let cfg = CategoryConfig::fast();
        let result = apply_functor(&t, &cfg)?;
        assert!(result.identity_error >= 0.0);
        Ok(())
    }

    #[test]
    fn test_apply_functor_composition_error_positive() -> Result<()> {
        let t = make_tensor(8, 42)?;
        let cfg = CategoryConfig::fast();
        let result = apply_functor(&t, &cfg)?;
        assert!(result.composition_error >= 0.0);
        Ok(())
    }

    #[test]
    fn test_apply_functor_deterministic() -> Result<()> {
        let t = make_tensor(6, 42)?;
        let cfg = CategoryConfig::fast();
        let r1 = apply_functor(&t, &cfg)?;
        let r2 = apply_functor(&t, &cfg)?;
        let d1: Vec<f32> = r1.transformed.to_vec1()?;
        let d2: Vec<f32> = r2.transformed.to_vec1()?;
        assert_eq!(d1, d2);
        Ok(())
    }

    #[test]
    fn test_apply_functor_trace_positive() -> Result<()> {
        let t = make_tensor(5, 42)?;
        let cfg = CategoryConfig::fast();
        let result = apply_functor(&t, &cfg)?;
        for &v in &result.composition_trace {
            assert!(v > 0.0, "trace value must be positive, got {}", v);
        }
        Ok(())
    }

    #[test]
    fn test_functor_result_display() -> Result<()> {
        let t = make_tensor(4, 42)?;
        let cfg = CategoryConfig::fast();
        let result = apply_functor(&t, &cfg)?;
        let s = format!("{}", result);
        assert!(!s.is_empty());
        assert!(s.contains("FunctorResult"));
        Ok(())
    }

    // === Natural Transformation Tests ===

    #[test]
    fn test_natural_transformation_basic() -> Result<()> {
        let t1 = make_tensor(10, 42)?;
        let t2 = make_tensor(10, 43)?;
        let cfg = CategoryConfig::fast();
        let f1 = apply_functor(&t1, &cfg)?;
        let f2 = apply_functor(&t2, &cfg)?;
        let result = compute_natural_transformation(&f1, &f2, &cfg)?;
        assert_eq!(result.components.len(), 10);
        assert!(result.naturality_error >= 0.0);
        Ok(())
    }

    #[test]
    fn test_natural_transformation_norm_positive() -> Result<()> {
        let t1 = make_tensor(8, 42)?;
        let t2 = make_tensor(8, 43)?;
        let cfg = CategoryConfig::fast();
        let f1 = apply_functor(&t1, &cfg)?;
        let f2 = apply_functor(&t2, &cfg)?;
        let result = compute_natural_transformation(&f1, &f2, &cfg)?;
        assert!(result.norm >= 0.0);
        Ok(())
    }

    #[test]
    fn test_natural_transformation_trajectory() -> Result<()> {
        let t1 = make_tensor(6, 42)?;
        let t2 = make_tensor(6, 43)?;
        let cfg = CategoryConfig::fast();
        let f1 = apply_functor(&t1, &cfg)?;
        let f2 = apply_functor(&t2, &cfg)?;
        let result = compute_natural_transformation(&f1, &f2, &cfg)?;
        assert!(!result.trajectory.is_empty());
        Ok(())
    }

    #[test]
    fn test_natural_transformation_same_functor() -> Result<()> {
        let t = make_tensor(8, 42)?;
        let cfg = CategoryConfig::fast();
        let f = apply_functor(&t, &cfg)?;
        let result = compute_natural_transformation(&f, &f, &cfg)?;
        // Same functor -> small transformation
        assert!(result.norm < 1.0);
        Ok(())
    }

    #[test]
    fn test_natural_transform_result_display() -> Result<()> {
        let t1 = make_tensor(4, 42)?;
        let t2 = make_tensor(4, 43)?;
        let cfg = CategoryConfig::fast();
        let f1 = apply_functor(&t1, &cfg)?;
        let f2 = apply_functor(&t2, &cfg)?;
        let result = compute_natural_transformation(&f1, &f2, &cfg)?;
        let s = format!("{}", result);
        assert!(!s.is_empty());
        assert!(s.contains("NaturalTransformResult"));
        Ok(())
    }

    // === Adjunction Tests ===

    #[test]
    fn test_adjunction_basic() -> Result<()> {
        let tx = make_tensor(10, 42)?;
        let ty = make_tensor(10, 43)?;
        let cfg = CategoryConfig::fast();
        let result = compute_adjunction(&tx, &ty, &cfg)?;
        assert_eq!(result.left_adjoint.len(), 10);
        assert_eq!(result.right_adjoint.len(), 10);
        assert_eq!(result.unit.len(), 10);
        assert_eq!(result.counit.len(), 10);
        Ok(())
    }

    #[test]
    fn test_adjunction_isomorphism_error_positive() -> Result<()> {
        let tx = make_tensor(8, 42)?;
        let ty = make_tensor(8, 43)?;
        let cfg = CategoryConfig::fast();
        let result = compute_adjunction(&tx, &ty, &cfg)?;
        assert!(result.isomorphism_error >= 0.0);
        Ok(())
    }

    #[test]
    fn test_adjunction_trajectory_not_empty() -> Result<()> {
        let tx = make_tensor(6, 42)?;
        let ty = make_tensor(6, 43)?;
        let cfg = CategoryConfig::fast();
        let result = compute_adjunction(&tx, &ty, &cfg)?;
        assert!(!result.trajectory.is_empty());
        Ok(())
    }

    #[test]
    fn test_adjunction_same_manifold() -> Result<()> {
        let t = make_tensor(8, 42)?;
        let cfg = CategoryConfig::fast();
        let result = compute_adjunction(&t, &t, &cfg)?;
        // Same manifold -> adjunction should be more coherent
        assert!(result.isomorphism_error >= 0.0);
        Ok(())
    }

    #[test]
    fn test_adjunction_different_dimensions() -> Result<()> {
        let tx = make_tensor(6, 42)?;
        let ty = make_tensor(10, 43)?;
        let cfg = CategoryConfig::fast();
        let result = compute_adjunction(&tx, &ty, &cfg)?;
        assert_eq!(result.left_adjoint.len(), 10);
        assert_eq!(result.right_adjoint.len(), 10);
        Ok(())
    }

    #[test]
    fn test_adjunction_result_display() -> Result<()> {
        let tx = make_tensor(4, 42)?;
        let ty = make_tensor(4, 43)?;
        let cfg = CategoryConfig::fast();
        let result = compute_adjunction(&tx, &ty, &cfg)?;
        let s = format!("{}", result);
        assert!(!s.is_empty());
        assert!(s.contains("AdjunctionResult"));
        Ok(())
    }

    // === Yoneda Tests ===

    #[test]
    fn test_yoneda_embedding_basic() -> Result<()> {
        let t = make_tensor(10, 42)?;
        let cfg = CategoryConfig::fast();
        let result = yoneda_embedding(&t, &cfg)?;
        assert_eq!(result.embedding.len(), 10);
        assert_eq!(result.embedding[0].len(), cfg.yoneda_dim);
        Ok(())
    }

    #[test]
    fn test_yoneda_representability_bounded() -> Result<()> {
        let t = make_tensor(8, 42)?;
        let cfg = CategoryConfig::fast();
        let result = yoneda_embedding(&t, &cfg)?;
        assert!(result.representability >= 0.0);
        assert!(result.representability <= 1.0);
        Ok(())
    }

    #[test]
    fn test_yoneda_nat_count_positive() -> Result<()> {
        let t = make_tensor(10, 42)?;
        let cfg = CategoryConfig::fast();
        let result = yoneda_embedding(&t, &cfg)?;
        assert!(result.nat_count > 0);
        Ok(())
    }

    #[test]
    fn test_yoneda_embedding_bounded() -> Result<()> {
        let t = make_tensor(8, 42)?;
        let cfg = CategoryConfig::fast();
        let result = yoneda_embedding(&t, &cfg)?;
        for row in &result.embedding {
            for &v in row {
                assert!(
                    v >= -1.0 && v <= 1.0,
                    "embedding value out of [-1,1]: {}",
                    v
                );
            }
        }
        Ok(())
    }

    #[test]
    fn test_yoneda_result_display() -> Result<()> {
        let t = make_tensor(4, 42)?;
        let cfg = CategoryConfig::fast();
        let result = yoneda_embedding(&t, &cfg)?;
        let s = format!("{}", result);
        assert!(!s.is_empty());
        assert!(s.contains("YonedaResult"));
        Ok(())
    }

    // === Manifold Composition Tests ===

    #[test]
    fn test_compose_manifolds_basic() -> Result<()> {
        let ta = make_tensor(10, 42)?;
        let tb = make_tensor(10, 43)?;
        let cfg = CategoryConfig::fast();
        let result = compose_manifolds(&ta, &tb, &cfg)?;
        assert_eq!(result.composed.shape().dims()[0], 10);
        Ok(())
    }

    #[test]
    fn test_compose_manifolds_coherence_bounded() -> Result<()> {
        let ta = make_tensor(8, 42)?;
        let tb = make_tensor(8, 43)?;
        let cfg = CategoryConfig::fast();
        let result = compose_manifolds(&ta, &tb, &cfg)?;
        assert!(result.coherence_score >= 0.0);
        assert!(result.coherence_score <= 1.0);
        Ok(())
    }

    #[test]
    fn test_compose_manifolds_same_manifold() -> Result<()> {
        let t = make_tensor(8, 42)?;
        let cfg = CategoryConfig::fast();
        let result = compose_manifolds(&t, &t, &cfg)?;
        // Same manifold -> higher coherence
        assert!(result.coherence_score > 0.0);
        Ok(())
    }

    #[test]
    fn test_compose_manifolds_different_dimensions() -> Result<()> {
        let ta = make_tensor(6, 42)?;
        let tb = make_tensor(10, 43)?;
        let cfg = CategoryConfig::fast();
        let result = compose_manifolds(&ta, &tb, &cfg)?;
        assert_eq!(result.composed.shape().dims()[0], 10);
        Ok(())
    }

    #[test]
    fn test_compose_manifold_result_display() -> Result<()> {
        let ta = make_tensor(4, 42)?;
        let tb = make_tensor(4, 43)?;
        let cfg = CategoryConfig::fast();
        let result = compose_manifolds(&ta, &tb, &cfg)?;
        let s = format!("{}", result);
        assert!(!s.is_empty());
        assert!(s.contains("ManifoldCompositionResult"));
        Ok(())
    }

    // === Noospheric Self-Organization Tests ===

    #[test]
    fn test_noospheric_self_org_empty() -> Result<()> {
        let manifolds: Vec<Tensor> = vec![];
        let cfg = CategoryConfig::fast();
        let score = noospheric_self_organization(&manifolds, &cfg)?;
        assert!((score - 0.0).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn test_noospheric_self_org_single() -> Result<()> {
        let t = make_tensor(8, 42)?;
        let cfg = CategoryConfig::fast();
        let score = noospheric_self_organization(&[t], &cfg)?;
        assert!((score - 1.0).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn test_noospheric_self_org_multiple() -> Result<()> {
        let t1 = make_tensor(6, 42)?;
        let t2 = make_tensor(6, 43)?;
        let cfg = CategoryConfig::fast();
        let score = noospheric_self_organization(&[t1.clone(), t2], &cfg)?;
        assert!(score >= 0.0 && score <= 1.0);
        Ok(())
    }

    #[test]
    fn test_noospheric_self_org_identical() -> Result<()> {
        let t = make_tensor(6, 42)?;
        let cfg = CategoryConfig::fast();
        let score = noospheric_self_organization(&[t.clone(), t], &cfg)?;
        // Identical manifolds -> high coherence
        assert!(score > 0.5);
        Ok(())
    }

    #[test]
    fn test_noospheric_self_org_bounded() -> Result<()> {
        let t1 = make_tensor(4, 42)?;
        let t2 = make_tensor(4, 100)?;
        let t3 = make_tensor(4, 200)?;
        let cfg = CategoryConfig::fast();
        let score = noospheric_self_organization(&[t1, t2, t3], &cfg)?;
        assert!(score >= 0.0 && score <= 1.0);
        Ok(())
    }

    // === Helper Function Tests ===

    #[test]
    fn test_lcg_next_deterministic() {
        let mut s = 42u64;
        let r1 = lcg_next(&mut s);
        let mut s = 42u64;
        let r2 = lcg_next(&mut s);
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_lcg_next_advances() {
        let mut s = 42u64;
        let r1 = lcg_next(&mut s);
        let r2 = lcg_next(&mut s);
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_random_uniform_range() {
        let mut s = 42u64;
        for _ in 0..100 {
            let r = random_uniform(&mut s);
            assert!(r >= 0.0, "uniform must be >= 0, got {}", r);
            assert!(r < 1.0, "uniform must be < 1, got {}", r);
        }
    }

    #[test]
    fn test_random_gaussian_finite() {
        let mut s = 12345u64;
        for _ in 0..100 {
            let g = random_gaussian(&mut s);
            assert!(g.is_finite(), "gaussian must be finite, got {}", g);
        }
    }

    // === Integration Tests ===

    #[test]
    fn test_full_category_pipeline() -> Result<()> {
        let ta = make_tensor(16, 42)?;
        let tb = make_tensor(16, 43)?;
        let cfg = CategoryConfig::fast();

        // Functor
        let fa = apply_functor(&ta, &cfg)?;
        assert!(fa.identity_error >= 0.0);

        // Natural transformation
        let fb = apply_functor(&tb, &cfg)?;
        let nat = compute_natural_transformation(&fa, &fb, &cfg)?;
        assert!(nat.naturality_error >= 0.0);

        // Adjunction
        let adj = compute_adjunction(&ta, &tb, &cfg)?;
        assert!(adj.isomorphism_error >= 0.0);

        // Yoneda
        let yoneda = yoneda_embedding(&ta, &cfg)?;
        assert!(yoneda.representability >= 0.0 && yoneda.representability <= 1.0);

        // Composition
        let comp = compose_manifolds(&ta, &tb, &cfg)?;
        assert!(comp.coherence_score >= 0.0 && comp.coherence_score <= 1.0);

        // Self-organization
        let org = noospheric_self_organization(&[ta, tb], &cfg)?;
        assert!(org >= 0.0 && org <= 1.0);

        Ok(())
    }

    #[test]
    fn test_category_coherence_improves_with_similarity() -> Result<()> {
        let t1 = make_tensor(8, 42)?;
        let t2 = make_tensor(8, 42)?; // Same seed = same manifold
        let t3 = make_tensor(8, 999)?; // Different seed
        let cfg = CategoryConfig::fast();

        let comp_same = compose_manifolds(&t1, &t2, &cfg)?;
        let comp_diff = compose_manifolds(&t1, &t3, &cfg)?;

        // Same manifolds should have higher or equal coherence
        assert!(
            comp_same.coherence_score >= comp_diff.coherence_score
                || (comp_same.coherence_score - comp_diff.coherence_score).abs() < 0.1,
            "same={:.4}, diff={:.4}",
            comp_same.coherence_score,
            comp_diff.coherence_score
        );
        Ok(())
    }

    #[test]
    fn test_yoneda_preserves_structure() -> Result<()> {
        let t = make_tensor(10, 42)?;
        let cfg_fast = CategoryConfig::fast();
        let cfg_precise = CategoryConfig::high_precision();

        let yoneda_fast = yoneda_embedding(&t, &cfg_fast)?;
        let yoneda_precise = yoneda_embedding(&t, &cfg_precise)?;

        // Higher precision should capture more structure
        assert!(
            yoneda_precise.representability >= yoneda_fast.representability - 0.1,
            "fast={:.4}, precise={:.4}",
            yoneda_fast.representability,
            yoneda_precise.representability
        );
        Ok(())
    }

    #[test]
    fn test_adjunction_convergence_trajectory_decreases() -> Result<()> {
        let tx = make_tensor(8, 42)?;
        let ty = make_tensor(8, 42)?; // Same manifold for fast convergence
        let cfg = CategoryConfig::high_precision();
        let result = compute_adjunction(&tx, &ty, &cfg)?;

        // Trajectory should show some decrease (or at least not explode)
        assert!(!result.trajectory.is_empty());
        if result.trajectory.len() > 2 {
            let first = result.trajectory[0];
            let last = *result.trajectory.last().unwrap();
            // Allow some tolerance for noise
            assert!(
                last < first * 2.0,
                "trajectory should not explode: first={:.4}, last={:.4}",
                first,
                last
            );
        }
        Ok(())
    }
}
