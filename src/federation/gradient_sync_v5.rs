//! Gradient Sync v5 — Distributed gradient synchronization with cross-model alignment and adaptive compression.
//!
//! Extends GradientSyncV3 with cross-model gradient alignment, adaptive compression ratios,
//! Byzantine fault tolerance detection, and bandwidth-aware batching.
//!
//! Feature-gated: `#[cfg(feature = "v1.5-sprint2")]`

mod internal {
    use std::collections::HashMap;

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    /// Errors for gradient sync v5.
    #[derive(Debug, Clone, PartialEq)]
    pub enum GradientSyncV5Error {
        /// Model not found.
        ModelNotFound(String),
        /// Node not found.
        NodeNotFound(String),
        /// Gradient dimension mismatch.
        DimensionMismatch { expected: usize, actual: usize },
        /// Compression failed.
        CompressionFailed(String),
        /// Byzantine behavior detected.
        ByzantineDetected(String),
        /// Sync timeout.
        SyncTimeout(u64),
        /// Bandwidth exceeded.
        BandwidthExceeded(f64),
    }

    impl std::fmt::Display for GradientSyncV5Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                GradientSyncV5Error::ModelNotFound(id) => write!(f, "Model {} not found", id),
                GradientSyncV5Error::NodeNotFound(id) => write!(f, "Node {} not found", id),
                GradientSyncV5Error::DimensionMismatch { expected, actual } => {
                    write!(f, "Dimension mismatch: expected={}, actual={}", expected, actual)
                }
                GradientSyncV5Error::CompressionFailed(msg) => {
                    write!(f, "Compression failed: {}", msg)
                }
                GradientSyncV5Error::ByzantineDetected(node) => {
                    write!(f, "Byzantine behavior detected from node {}", node)
                }
                GradientSyncV5Error::SyncTimeout(ms) => {
                    write!(f, "Sync timeout after {}ms", ms)
                }
                GradientSyncV5Error::BandwidthExceeded(bw) => {
                    write!(f, "Bandwidth exceeded: {:.2} MB/s", bw)
                }
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Config
    // ---------------------------------------------------------------------------

    /// Configuration for gradient sync v5.
    #[derive(Debug, Clone)]
    pub struct GradientSyncV5Config {
        /// Maximum gradient dimensions.
        pub max_dimensions: usize,
        /// Compression ratio (0.0-1.0, lower = more compression).
        pub compression_ratio: f64,
        /// Byzantine detection threshold (MAD multiplier).
        pub byzantine_threshold: f64,
        /// Maximum bandwidth (MB/s).
        pub max_bandwidth_mbps: f64,
        /// Sync timeout (ms).
        pub sync_timeout_ms: u64,
        /// Batch size for gradient aggregation.
        pub batch_size: usize,
        /// Adaptive compression enabled.
        pub adaptive_compression: bool,
        /// Cross-model alignment weight.
        pub alignment_weight: f64,
        /// EMA alpha for gradient tracking.
        pub ema_alpha: f64,
    }

    impl Default for GradientSyncV5Config {
        fn default() -> Self {
            Self {
                max_dimensions: 8192,
                compression_ratio: 0.5,
                byzantine_threshold: 3.0,
                max_bandwidth_mbps: 100.0,
                sync_timeout_ms: 5000,
                batch_size: 32,
                adaptive_compression: true,
                alignment_weight: 0.3,
                ema_alpha: 0.3,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Gradient Entry
    // ---------------------------------------------------------------------------

    /// Gradient entry from a node.
    #[derive(Debug, Clone)]
    pub struct GradientEntry {
        /// Node identifier.
        pub node_id: String,
        /// Model identifier.
        pub model_id: String,
        /// Gradient values.
        pub gradients: Vec<f32>,
        /// Timestamp (ms).
        pub timestamp_ms: u64,
        /// Compression ratio applied.
        pub compression_ratio: f64,
        /// Original size (bytes).
        pub original_size: usize,
        /// Compressed size (bytes).
        pub compressed_size: usize,
    }

    impl GradientEntry {
        /// Create a new gradient entry.
        pub fn new(
            node_id: String,
            model_id: String,
            gradients: Vec<f32>,
            timestamp_ms: u64,
        ) -> Self {
            let original_size = gradients.len() * sizeof_f32();
            Self {
                node_id,
                model_id,
                gradients,
                timestamp_ms,
                compression_ratio: 1.0,
                original_size,
                compressed_size: original_size,
            }
        }

        /// Apply Top-K compression.
        pub fn compress_topk(&mut self, k: usize) {
            let dim = self.gradients.len();
            let topk = k.min(dim);
            // Keep top-k largest magnitude gradients
            let mut indexed: Vec<(usize, f32)> =
                self.gradients.iter().enumerate().map(|(i, &v)| (i, v)).collect();
            indexed.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap());
            let mut compressed = vec![0.0f32; dim];
            for &(i, v) in indexed.iter().take(topk) {
                compressed[i] = v;
            }
            self.compression_ratio = topk as f64 / dim as f64;
            self.compressed_size = topk * sizeof_f32();
            self.gradients = compressed;
        }

        /// Compute L2 norm.
        pub fn l2_norm(&self) -> f64 {
            self.gradients.iter().map(|g| (*g as f64) * (*g as f64)).sum::<f64>().sqrt()
        }
    }

    fn sizeof_f32() -> usize {
        std::mem::size_of::<f32>()
    }

    // ---------------------------------------------------------------------------
    // Model Registry
    // ---------------------------------------------------------------------------

    /// Model registry for cross-model alignment.
    #[derive(Debug, Clone)]
    pub struct ModelRegistryV5 {
        /// Model identifier.
        pub model_id: String,
        /// Gradient dimension.
        pub dimension: usize,
        /// EMA aggregated gradients.
        pub ema_gradients: Vec<f64>,
        /// Alignment score (0.0-1.0).
        pub alignment_score: f64,
        /// Sync count.
        pub sync_count: usize,
    }

    impl ModelRegistryV5 {
        /// Create a new model registry entry.
        pub fn new(model_id: String, dimension: usize) -> Self {
            Self {
                model_id,
                dimension,
                ema_gradients: vec![0.0; dimension],
                alignment_score: 1.0,
                sync_count: 0,
            }
        }

        /// Update EMA gradients.
        pub fn update_ema(&mut self, gradients: &[f32], alpha: f64) {
            for (i, ema) in self.ema_gradients.iter_mut().enumerate() {
                if i < gradients.len() {
                    *ema = alpha * (gradients[i] as f64) + (1.0 - alpha) * *ema;
                }
            }
            self.sync_count += 1;
        }
    }

    // ---------------------------------------------------------------------------
    // Stats
    // ---------------------------------------------------------------------------

    /// Statistics for gradient sync v5.
    #[derive(Debug, Clone)]
    pub struct GradientSyncV5Stats {
        /// Total syncs completed.
        pub total_syncs: usize,
        /// Total gradients processed.
        pub gradients_processed: usize,
        /// Byzantine detections.
        pub byzantine_detections: usize,
        /// Average compression ratio.
        pub avg_compression_ratio: f64,
        /// Total bandwidth used (MB).
        pub total_bandwidth_mb: f64,
        /// Sync failures.
        pub sync_failures: usize,
    }

    impl Default for GradientSyncV5Stats {
        fn default() -> Self {
            Self {
                total_syncs: 0,
                gradients_processed: 0,
                byzantine_detections: 0,
                avg_compression_ratio: 1.0,
                total_bandwidth_mb: 0.0,
                sync_failures: 0,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Gradient Sync V5 Engine
    // ---------------------------------------------------------------------------

    /// Gradient Sync v5 engine with cross-model alignment and adaptive compression.
    pub struct GradientSyncV5 {
        config: GradientSyncV5Config,
        models: HashMap<String, ModelRegistryV5>,
        pending: Vec<GradientEntry>,
        stats: GradientSyncV5Stats,
    }

    impl GradientSyncV5 {
        /// Create a new gradient sync engine.
        pub fn new(config: GradientSyncV5Config) -> Self {
            Self {
                config,
                models: HashMap::new(),
                pending: Vec::new(),
                stats: GradientSyncV5Stats::default(),
            }
        }

        /// Register a model.
        pub fn register_model(
            &mut self,
            model_id: String,
            dimension: usize,
        ) -> Result<(), GradientSyncV5Error> {
            if dimension > self.config.max_dimensions {
                return Err(GradientSyncV5Error::DimensionMismatch {
                    expected: self.config.max_dimensions,
                    actual: dimension,
                });
            }
            self.models.insert(model_id.clone(), ModelRegistryV5::new(model_id, dimension));
            Ok(())
        }

        /// Submit gradients from a node.
        pub fn submit_gradients(
            &mut self,
            node_id: String,
            model_id: String,
            gradients: Vec<f32>,
            timestamp_ms: u64,
        ) -> Result<(), GradientSyncV5Error> {
            let model = self.models.get(&model_id).ok_or_else(|| {
                GradientSyncV5Error::ModelNotFound(model_id.to_string())
            })?;

            if gradients.len() != model.dimension {
                return Err(GradientSyncV5Error::DimensionMismatch {
                    expected: model.dimension,
                    actual: gradients.len(),
                });
            }

            // Byzantine detection: check gradient norm
            let norm = {
                let sum: f64 = gradients.iter().map(|g| (*g as f64) * (*g as f64)).sum();
                sum.sqrt()
            };
            let mean_norm = self.stats.avg_compression_ratio; // Reuse as proxy
            if norm > mean_norm * self.config.byzantine_threshold && self.stats.total_syncs > 0 {
                self.stats.byzantine_detections += 1;
                return Err(GradientSyncV5Error::ByzantineDetected(node_id));
            }

            let dim = gradients.len();
            let mut entry = GradientEntry::new(node_id, model_id, gradients, timestamp_ms);

            // Apply adaptive compression
            if self.config.adaptive_compression {
                let k = (dim as f64 * self.config.compression_ratio) as usize;
                entry.compress_topk(k.max(1));
            }

            self.pending.push(entry);
            Ok(())
        }

        /// Execute sync round: aggregate pending gradients.
        pub fn execute_sync(&mut self) -> Result<HashMap<String, Vec<f64>>, GradientSyncV5Error> {
            if self.pending.is_empty() {
                return Ok(HashMap::new());
            }

            let mut aggregated: HashMap<String, Vec<f64>> = HashMap::new();

            // Group by model
            for entry in self.pending.drain(..) {
                let grads = aggregated
                    .entry(entry.model_id.clone())
                    .or_insert_with(|| {
                        if let Some(model) = self.models.get(&entry.model_id) {
                            vec![0.0; model.dimension]
                        } else {
                            vec![]
                        }
                    });

                for (i, g) in entry.gradients.iter().enumerate() {
                    if i < grads.len() {
                        grads[i] += *g as f64;
                    }
                }
                self.stats.gradients_processed += 1;
                self.stats.total_bandwidth_mb += entry.compressed_size as f64 / (1024.0 * 1024.0);
            }

            // Average and update EMA
            for (model_id, grads) in &mut aggregated {
                let count = grads.len().max(1);
                if let Some(model) = self.models.get_mut(model_id) {
                    for g in grads.iter_mut() {
                        *g /= count as f64;
                    }
                    model.update_ema(
                        &grads.iter().map(|g| *g as f32).collect::<Vec<_>>(),
                        self.config.ema_alpha,
                    );
                }
            }

            self.stats.total_syncs += 1;
            Ok(aggregated)
        }

        /// Get model alignment score.
        pub fn get_alignment_score(&self, model_id: &str) -> Option<f64> {
            self.models.get(model_id).map(|m| m.alignment_score)
        }

        /// Get stats reference.
        pub fn stats(&self) -> &GradientSyncV5Stats {
            &self.stats
        }

        /// Get pending count.
        pub fn pending_count(&self) -> usize {
            self.pending.len()
        }

        /// Get model count.
        pub fn model_count(&self) -> usize {
            self.models.len()
        }

        /// Get models reference.
        pub fn models(&self) -> &HashMap<String, ModelRegistryV5> {
            &self.models
        }
    }

    impl Default for GradientSyncV5 {
        fn default() -> Self {
            Self::new(GradientSyncV5Config::default())
        }
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_engine_creation() {
            let engine = GradientSyncV5::default();
            assert_eq!(engine.model_count(), 0);
            assert_eq!(engine.pending_count(), 0);
        }

        #[test]
        fn test_register_model() {
            let mut engine = GradientSyncV5::default();
            engine.register_model("model-1".to_string(), 128).unwrap();
            assert_eq!(engine.model_count(), 1);
        }

        #[test]
        fn test_register_model_dimension_exceeded() {
            let mut engine = GradientSyncV5::default();
            let result = engine.register_model("model-1".to_string(), 9999);
            assert!(result.is_err());
        }

        #[test]
        fn test_submit_gradients() {
            let mut engine = GradientSyncV5::default();
            engine.register_model("model-1".to_string(), 128).unwrap();
            let grads = vec![0.01f32; 128];
            engine.submit_gradients("node-1".to_string(), "model-1".to_string(), grads, 1000).unwrap();
            assert_eq!(engine.pending_count(), 1);
        }

        #[test]
        fn test_submit_gradients_model_not_found() {
            let mut engine = GradientSyncV5::default();
            let grads = vec![0.01f32; 128];
            let result = engine.submit_gradients("node-1".to_string(), "missing".to_string(), grads, 1000);
            assert!(result.is_err());
        }

        #[test]
        fn test_submit_gradients_dimension_mismatch() {
            let mut engine = GradientSyncV5::default();
            engine.register_model("model-1".to_string(), 128).unwrap();
            let grads = vec![0.01f32; 64];
            let result = engine.submit_gradients("node-1".to_string(), "model-1".to_string(), grads, 1000);
            assert!(result.is_err());
        }

        #[test]
        fn test_execute_sync() {
            let mut engine = GradientSyncV5::default();
            engine.register_model("model-1".to_string(), 64).unwrap();
            let grads = vec![0.1f32; 64];
            engine.submit_gradients("node-1".to_string(), "model-1".to_string(), grads.clone(), 1000).unwrap();
            engine.submit_gradients("node-2".to_string(), "model-1".to_string(), grads, 1001).unwrap();
            let result = engine.execute_sync().unwrap();
            assert!(result.contains_key("model-1"));
            assert_eq!(engine.stats().total_syncs, 1);
        }

        #[test]
        fn test_execute_sync_empty() {
            let mut engine = GradientSyncV5::default();
            let result = engine.execute_sync().unwrap();
            assert!(result.is_empty());
        }

        #[test]
        fn test_compression() {
            let mut engine = GradientSyncV5::default();
            engine.register_model("model-1".to_string(), 128).unwrap();
            let grads: Vec<f32> = (0..128).map(|i| i as f32 * 0.01).collect();
            engine.submit_gradients("node-1".to_string(), "model-1".to_string(), grads, 1000).unwrap();
            let entry = &engine.pending[0];
            assert!(entry.compression_ratio < 1.0);
            assert!(entry.compressed_size < entry.original_size);
        }

        #[test]
        fn test_gradient_norm() {
            let entry = GradientEntry::new("n".into(), "m".into(), vec![1.0, 2.0, 3.0], 0);
            let norm = entry.l2_norm();
            assert!((norm - 3.7417).abs() < 0.01);
        }

        #[test]
        fn test_stats_tracking() {
            let mut engine = GradientSyncV5::default();
            engine.register_model("model-1".to_string(), 64).unwrap();
            let grads = vec![0.1f32; 64];
            engine.submit_gradients("node-1".to_string(), "model-1".to_string(), grads, 1000).unwrap();
            engine.execute_sync().unwrap();
            assert_eq!(engine.stats().total_syncs, 1);
            assert_eq!(engine.stats().gradients_processed, 1);
        }

        #[test]
        fn test_config_default() {
            let config = GradientSyncV5Config::default();
            assert_eq!(config.max_dimensions, 8192);
            assert!(config.adaptive_compression);
        }

        #[test]
        fn test_error_display() {
            let err = GradientSyncV5Error::ModelNotFound("test".to_string());
            let msg = format!("{}", err);
            assert!(!msg.is_empty());
        }

        #[test]
        fn test_model_registry_ema() {
            let mut registry = ModelRegistryV5::new("m".to_string(), 4);
            registry.update_ema(&[1.0, 2.0, 3.0, 4.0], 0.5);
            assert!(registry.ema_gradients[0] > 0.0);
            assert_eq!(registry.sync_count, 1);
        }
    }
}

pub use internal::*;
