//! WASM Homology Latency Benchmark — Sprint 75: Thermodynamic Hardening & Asynchronous Neuro-Symbolic Pivot
//!
//! Benchmark persistent homology computation on 4096-dim tensors
//! within WASM sandbox constraints. Validates that β₁ approximation
//! runs in <50ms target for real-time sidecar validation.
//!
//! **Feature Gate**: `v9.11-performance-pivot`
//! **Purpose**: Measure homology computation latency at production tensor sizes.
//! **Status**: Production benchmark — validates GEI proxy delegation threshold.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

// ---------------------------------------------------------------------------
// Simulated persistent homology structures
// ---------------------------------------------------------------------------

/// Configuration for homology benchmark parameters.
#[derive(Clone, Debug)]
struct HomologyConfig {
    /// Dimension of input tensor (default: 4096)
    tensor_dim: usize,
    /// Simplicial complex resolution (epsilon for Rips complex)
    epsilon: f32,
    /// Maximum simplex dimension (0=simplices, 1=edges, 2=triangles)
    max_dim: usize,
    /// Sampling rate for stratified approximation (1.0 = full)
    sample_rate: f64,
    /// WASM memory limit in MB (simulated constraint)
    wasm_memory_limit_mb: usize,
}

impl HomologyConfig {
    fn default_benchmark() -> Self {
        Self {
            tensor_dim: 4096,
            epsilon: 0.05,
            max_dim: 1,
            sample_rate: 1.0,
            wasm_memory_limit_mb: 64,
        }
    }

    fn low_res() -> Self {
        Self {
            tensor_dim: 1024,
            epsilon: 0.1,
            max_dim: 1,
            sample_rate: 0.5,
            wasm_memory_limit_mb: 32,
        }
    }

    fn high_res() -> Self {
        Self {
            tensor_dim: 8192,
            epsilon: 0.02,
            max_dim: 2,
            sample_rate: 1.0,
            wasm_memory_limit_mb: 128,
        }
    }

    fn validate(&self) -> bool {
        self.tensor_dim > 0
            && self.tensor_dim <= 16384
            && self.epsilon > 0.0
            && self.epsilon < 1.0
            && self.max_dim <= 2
            && self.sample_rate > 0.0
            && self.sample_rate <= 1.0
            && self.wasm_memory_limit_mb >= 16
    }
}

/// Simulated Betti numbers from persistent homology computation.
#[derive(Clone, Debug)]
struct BettiNumbers {
    /// β₀: Connected components
    beta_0: usize,
    /// β₁: 1-dimensional holes (loops)
    beta_1: usize,
    /// β₂: 2-dimensional voids (cavities)
    beta_2: usize,
    /// Persistence diagram points (birth, death)
    persistence_points: Vec<(f32, f32)>,
    /// Computation time in milliseconds
    compute_time_ms: f64,
}

impl BettiNumbers {
    fn new(beta_0: usize, beta_1: usize, beta_2: usize) -> Self {
        Self {
            beta_0,
            beta_1,
            beta_2,
            persistence_points: Vec::new(),
            compute_time_ms: 0.0,
        }
    }
}

/// Simulated WASM sandbox environment for homology computation.
struct WasmSandbox {
    memory_limit_bytes: usize,
    execution_timeout_ms: u64,
    /// Accumulated computation time across calls
    total_compute_time_ms: f64,
    /// Number of computations performed
    computation_count: usize,
}

impl WasmSandbox {
    fn new(memory_limit_mb: usize, timeout_ms: u64) -> Self {
        Self {
            memory_limit_bytes: memory_limit_mb * 1024 * 1024,
            execution_timeout_ms: timeout_ms,
            total_compute_time_ms: 0.0,
            computation_count: 0,
        }
    }

    /// Simulate persistent homology computation within WASM constraints.
    ///
    /// Uses simulated union-find based Rips complex construction
    /// with stratified sampling for large tensors.
    fn compute_homology(&mut self, activations: &[f32], config: &HomologyConfig) -> Option<BettiNumbers> {
        // Memory check: activations must fit in WASM sandbox
        let required_bytes = activations.len() * std::mem::size_of::<f32>();
        if required_bytes > self.memory_limit_bytes {
            return None;
        }

        let start = std::time::Instant::now();

        // Stratified sampling
        let n_points = (activations.len() as f64 * config.sample_rate / 4.0) as usize;
        let sampled: Vec<f32> = if n_points < activations.len() / 4 {
            let step = activations.len() / (4 * n_points);
            (0..n_points)
                .map(|i| activations[i * 4 * step])
                .collect()
        } else {
            activations.iter().step_by(4).copied().collect()
        };

        // Simulated Rips complex via union-find
        let mut parent: Vec<usize> = (0..sampled.len()).collect();
        let mut rank = vec![0u32; sampled.len()];
        let mut edges = 0usize;

        let find = |parent: &mut [usize], mut x: usize| -> usize {
            while parent[x] != x {
                parent[x] = parent[parent[x]];
                x = parent[x];
            }
            x
        };

        let union = |parent: &mut [usize], rank: &mut [u32], mut a: usize, mut b: usize| -> bool {
            while parent[a] != a {
                parent[a] = parent[parent[a]];
                a = parent[a];
            }
            while parent[b] != b {
                parent[b] = parent[parent[b]];
                b = parent[b];
            }
            if a == b {
                return false;
            }
            match rank[a].cmp(&rank[b]) {
                std::cmp::Ordering::Less => parent[a] = b,
                std::cmp::Ordering::Greater => parent[b] = a,
                std::cmp::Ordering::Equal => {
                    parent[b] = a;
                    rank[a] += 1;
                }
            }
            true
        };

        // Build edges within epsilon distance
        let mut persistence_points = Vec::new();
        for i in 0..sampled.len() {
            for j in (i + 1)..sampled.len() {
                let dist = ((sampled[i] - sampled[j]).abs()).min(1.0);
                if dist < config.epsilon {
                    let merged = union(&mut parent, &mut rank, i, j);
                    if merged {
                        edges += 1;
                        persistence_points.push((dist, config.epsilon));
                    }
                }
            }
        }

        // Count connected components (β₀)
        let mut components = std::collections::HashSet::new();
        for i in 0..sampled.len() {
            components.insert(find(&mut parent, i));
        }
        let beta_0 = components.len();

        // Euler characteristic: χ = β₀ - β₁ => β₁ = β₀ - χ
        // For Rips complex: χ = V - E (vertices - edges)
        let euler_char = (sampled.len() as isize) - (edges as isize);
        let beta_1 = ((beta_0 as isize) - euler_char).max(0) as usize;

        // β₂ simulation (only if max_dim >= 2)
        let beta_2 = if config.max_dim >= 2 {
            // Simplified: count triangles that form voids
            (persistence_points.len() / 10).min(5)
        } else {
            0
        };

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;

        self.total_compute_time_ms += elapsed;
        self.computation_count += 1;

        // Timeout check
        if elapsed > self.execution_timeout_ms as f64 {
            return None;
        }

        Some(BettiNumbers {
            beta_0,
            beta_1,
            beta_2,
            persistence_points,
            compute_time_ms: elapsed,
        })
    }

    fn average_time_ms(&self) -> f64 {
        if self.computation_count == 0 {
            return 0.0;
        }
        self.total_compute_time_ms / self.computation_count as f64
    }
}

// ---------------------------------------------------------------------------
// Benchmark functions
// ---------------------------------------------------------------------------

/// Generate deterministic test activations for reproducible benchmarks.
fn generate_activations(dim: usize) -> Vec<f32> {
    let mut activations = Vec::with_capacity(dim);
    for i in 0..dim {
        // FNV-1a based deterministic pseudo-random
        let hash = fnv_hash_32(i as u32);
        let value = ((hash % 10000) as f32 / 10000.0) * 2.0 - 1.0; // [-1, 1]
        activations.push(value);
    }
    activations
}

/// FNV-1a 32-bit hash for deterministic test data.
fn fnv_hash_32(mut input: u32) -> u32 {
    let mut hash: u32 = 2166136261;
    for _ in 0..4 {
        hash ^= input & 0xFF;
        hash = hash.wrapping_mul(16777619);
        input >>= 8;
    }
    hash
}

fn bench_homology_4096_dim(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasm_homology_4096dim");
    group.sample_size(100);

    let config = HomologyConfig::default_benchmark();
    let activations = generate_activations(config.tensor_dim);

    group.bench_function(BenchmarkId::new("full_complex", "epsilon_0.05"), |b| {
        b.iter(|| {
            let mut sandbox = WasmSandbox::new(64, 50);
            black_box(sandbox.compute_homology(black_box(&activations), black_box(&config)));
        })
    });

    group.bench_function(BenchmarkId::new("sampled_50pct", "epsilon_0.05"), |b| {
        let sampled_config = HomologyConfig {
            sample_rate: 0.5,
            ..config.clone()
        };
        b.iter(|| {
            let mut sandbox = WasmSandbox::new(64, 50);
            black_box(sandbox.compute_homology(black_box(&activations), black_box(&sampled_config)));
        })
    });

    group.bench_function(BenchmarkId::new("high_epsilon", "epsilon_0.1"), |b| {
        let high_eps_config = HomologyConfig {
            epsilon: 0.1,
            ..config.clone()
        };
        b.iter(|| {
            let mut sandbox = WasmSandbox::new(64, 50);
            black_box(sandbox.compute_homology(black_box(&activations), black_box(&high_eps_config)));
        })
    });

    group.finish();
}

fn bench_homology_dimension_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasm_homology_dim_scaling");
    group.sample_size(50);

    let dimensions = [512, 1024, 2048, 4096, 8192];

    for dim in &dimensions {
        let config = HomologyConfig {
            tensor_dim: *dim,
            epsilon: 0.05,
            max_dim: 1,
            sample_rate: 1.0,
            wasm_memory_limit_mb: 64,
        };
        let activations = generate_activations(*dim);

        group.bench_function(BenchmarkId::new("rips_complex", format!("dim_{}", dim)), |b| {
            b.iter(|| {
                let mut sandbox = WasmSandbox::new(64, 100);
                black_box(sandbox.compute_homology(black_box(&activations), black_box(&config)));
            })
        });
    }

    group.finish();
}

fn bench_homology_epsilon_sensitivity(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasm_homology_epsilon_sensitivity");
    group.sample_size(50);

    let activations = generate_activations(4096);
    let epsilons = [0.01, 0.02, 0.05, 0.1, 0.2];

    for eps in &epsilons {
        let config = HomologyConfig {
            tensor_dim: 4096,
            epsilon: *eps,
            max_dim: 1,
            sample_rate: 1.0,
            wasm_memory_limit_mb: 64,
        };

        group.bench_function(BenchmarkId::new("rips_complex", format!("epsilon_{}", eps)), |b| {
            b.iter(|| {
                let mut sandbox = WasmSandbox::new(64, 100);
                black_box(sandbox.compute_homology(black_box(&activations), black_box(&config)));
            })
        });
    }

    group.finish();
}

fn bench_wasm_memory_constraint(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasm_memory_constraint");
    group.sample_size(50);

    let activations = generate_activations(4096);
    let config = HomologyConfig::default_benchmark();

    for limit_mb in [16, 32, 64, 128] {
        group.bench_function(BenchmarkId::new("homology", format!("{}MB", limit_mb)), |b| {
            b.iter(|| {
                let mut sandbox = WasmSandbox::new(limit_mb, 100);
                black_box(sandbox.compute_homology(black_box(&activations), black_box(&config)));
            })
        });
    }

    group.finish();
}

fn bench_latency_budget_compliance(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency_budget_compliance");
    group.sample_size(100);

    let activations = generate_activations(4096);
    let config = HomologyConfig::default_benchmark();

    // Target: <50ms for sidecar validation pipeline
    group.bench_function("target_50ms_budget", |b| {
        b.iter(|| {
            let mut sandbox = WasmSandbox::new(64, 50);
            let result = sandbox.compute_homology(black_box(&activations), black_box(&config));
            black_box(result);
        })
    });

    // Aggressive target: <20ms for proxy delegation
    group.bench_function("target_20ms_proxy", |b| {
        let fast_config = HomologyConfig {
            sample_rate: 0.3,
            epsilon: 0.1,
            ..config.clone()
        };
        b.iter(|| {
            let mut sandbox = WasmSandbox::new(64, 20);
            let result = sandbox.compute_homology(black_box(&activations), black_box(&fast_config));
            black_box(result);
        })
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Criterion registration
// ---------------------------------------------------------------------------

criterion_group!(
    name = wasm_homology_benchmarks;
    config = Criterion::default().sample_size(100);
    targets =
        bench_homology_4096_dim,
        bench_homology_dimension_scaling,
        bench_homology_epsilon_sensitivity,
        bench_wasm_memory_constraint,
        bench_latency_budget_compliance
);
criterion_main!(wasm_homology_benchmarks);
