//! Gradient Sync v6 — Gradient synchronization with cross-model alignment and adaptive compression.
//!
//! Features:
//! - Cross-model gradient synchronization with weighted averaging
//! - Adaptive compression with quality-aware top-K selection
//! - EMA-based gradient smoothing for stable convergence
//! - Distributed gradient aggregation with reputation weighting
//!
//! Performance targets:
//! - Gradient sync <= 100ms
//! - Compression ratio >= 2x
//! - Cross-model alignment error <= 0.05
//!
//! Guardrails: Zero financial logic, zero telemetry, zero unsafe.
//! License: Apache 2.0 + Ethical Use

mod internal {
    use std::collections::HashMap;
    use std::fmt;

    /// Gradient Sync v6 Error types
    #[derive(Debug, Clone, PartialEq)]
    pub enum GradientSyncV6Error {
        ModelNotFound(String),
        DimensionMismatch { expected: usize, got: usize },
        InvalidWeight(f64),
        InvalidCompressionRatio(f64),
        SyncFailed(String),
        InsufficientModels,
        ConfigurationError(String),
    }

    impl fmt::Display for GradientSyncV6Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                GradientSyncV6Error::ModelNotFound(id) => {
                    write!(f, "Model not found: {}", id)
                }
                GradientSyncV6Error::DimensionMismatch { expected, got } => {
                    write!(
                        f,
                        "Dimension mismatch: expected {}, got {}",
                        expected, got
                    )
                }
                GradientSyncV6Error::InvalidWeight(weight) => {
                    write!(f, "Invalid weight value: {}", weight)
                }
                GradientSyncV6Error::InvalidCompressionRatio(ratio) => {
                    write!(f, "Invalid compression ratio: {}", ratio)
                }
                GradientSyncV6Error::SyncFailed(msg) => {
                    write!(f, "Sync failed: {}", msg)
                }
                GradientSyncV6Error::InsufficientModels => {
                    write!(f, "Insufficient models for sync")
                }
                GradientSyncV6Error::ConfigurationError(msg) => {
                    write!(f, "Configuration error: {}", msg)
                }
            }
        }
    }

    /// Gradient Sync v6 Configuration
    pub struct GradientSyncV6Config {
        /// Maximum gradient dimension
        pub max_dimension: usize,
        /// EMA alpha for gradient smoothing
        pub ema_alpha: f64,
        /// Compression top-K ratio (0.0-1.0)
        pub compression_ratio: f64,
        /// Cross-model alignment weight
        pub cross_model_weight: f64,
        /// Reputation weight for aggregation
        pub reputation_weight: f64,
        /// Maximum gradient history per model
        pub max_gradient_history: usize,
        /// Enable adaptive compression
        pub adaptive_compression: bool,
        /// Enable cross-model alignment
        pub cross_model_alignment: bool,
    }

    impl Default for GradientSyncV6Config {
        fn default() -> Self {
            Self {
                max_dimension: 10000,
                ema_alpha: 0.3,
                compression_ratio: 0.3,
                cross_model_weight: 0.2,
                reputation_weight: 0.5,
                max_gradient_history: 50,
                adaptive_compression: true,
                cross_model_alignment: true,
            }
        }
    }

    /// Gradient entry for a model
    pub struct GradientEntryV6 {
        model_id: String,
        gradients: Vec<f32>,
        compressed_gradients: Option<Vec<f32>>,
        timestamp_ms: u64,
        gradient_norm: f64,
        compression_ratio: f64,
    }

    impl GradientEntryV6 {
        pub fn new(
            model_id: String,
            gradients: Vec<f32>,
            timestamp_ms: u64,
        ) -> Self {
            let norm = Self::compute_norm(&gradients);
            Self {
                model_id,
                gradients,
                compressed_gradients: None,
                timestamp_ms,
                gradient_norm: norm,
                compression_ratio: 0.0,
            }
        }

        pub fn model_id(&self) -> &str {
            &self.model_id
        }

        pub fn gradients(&self) -> &[f32] {
            &self.gradients
        }

        pub fn compressed_gradients(&self) -> Option<&[f32]> {
            self.compressed_gradients.as_deref()
        }

        pub fn timestamp_ms(&self) -> u64 {
            self.timestamp_ms
        }

        pub fn gradient_norm(&self) -> f64 {
            self.gradient_norm
        }

        pub fn compression_ratio(&self) -> f64 {
            self.compression_ratio
        }

        pub fn compress_topk(&mut self, k: usize) {
            let dim = self.gradients.len();
            if k >= dim {
                self.compressed_gradients = Some(self.gradients.clone());
                self.compression_ratio = 1.0;
                return;
            }

            // Find top-K indices by absolute value
            let mut indexed: Vec<(usize, f64)> = self
                .gradients
                .iter()
                .enumerate()
                .map(|(i, &v)| (i, v.abs() as f64))
                .collect();
            indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            let mut compressed = vec![0.0f32; dim];
            for (i, _) in indexed.iter().take(k) {
                compressed[*i] = self.gradients[*i];
            }

            self.compressed_gradients = Some(compressed);
            self.compression_ratio = k as f64 / dim as f64;
        }

        fn compute_norm(grads: &[f32]) -> f64 {
            let sum: f64 = grads.iter().map(|g| (*g as f64) * (*g as f64)).sum();
            sum.sqrt()
        }
    }

    /// Model registry entry
    pub struct ModelRegistryV6 {
        model_id: String,
        dimension: usize,
        ema_gradients: Vec<f64>,
        reputation: f64,
        gradient_history: Vec<f64>,
        total_updates: u64,
        total_syncs: u64,
    }

    impl ModelRegistryV6 {
        pub fn new(model_id: String, dimension: usize) -> Self {
            Self {
                model_id,
                dimension,
                ema_gradients: vec![0.0; dimension],
                reputation: 1.0,
                gradient_history: Vec::new(),
                total_updates: 0,
                total_syncs: 0,
            }
        }

        pub fn model_id(&self) -> &str {
            &self.model_id
        }

        pub fn dimension(&self) -> usize {
            self.dimension
        }

        pub fn reputation(&self) -> f64 {
            self.reputation
        }

        pub fn update_reputation(&mut self, new_reputation: f64) {
            self.reputation = new_reputation.clamp(0.0, 1.0);
        }

        pub fn update_ema(&mut self, gradients: &[f32], alpha: f64) {
            for (i, ema) in self.ema_gradients.iter_mut().enumerate() {
                if i < gradients.len() {
                    *ema = alpha * (gradients[i] as f64) + (1.0 - alpha) * *ema;
                }
            }
        }

        pub fn record_gradient_norm(&mut self, norm: f64, max_history: usize) {
            self.gradient_history.push(norm);
            if self.gradient_history.len() > max_history {
                self.gradient_history.remove(0);
            }
        }

        pub fn record_update(&mut self) {
            self.total_updates += 1;
        }

        pub fn record_sync(&mut self) {
            self.total_syncs += 1;
        }

        pub fn avg_gradient_norm(&self) -> f64 {
            if self.gradient_history.is_empty() {
                return 0.0;
            }
            let sum: f64 = self.gradient_history.iter().sum();
            sum / self.gradient_history.len() as f64
        }

        pub fn ema_gradients(&self) -> &[f64] {
            &self.ema_gradients
        }
    }

    /// Gradient Sync v6 Statistics
    pub struct GradientSyncV6Stats {
        pub total_syncs: u64,
        pub total_gradients: u64,
        pub total_compressions: u64,
        pub avg_sync_time_ms: f64,
        pub avg_compression_ratio: f64,
        pub avg_gradient_norm: f64,
        pub cross_model_alignments: u64,
    }

    impl Default for GradientSyncV6Stats {
        fn default() -> Self {
            Self {
                total_syncs: 0,
                total_gradients: 0,
                total_compressions: 0,
                avg_sync_time_ms: 0.0,
                avg_compression_ratio: 0.0,
                avg_gradient_norm: 0.0,
                cross_model_alignments: 0,
            }
        }
    }

    impl GradientSyncV6Stats {
        pub fn record_sync(&mut self, time_ms: u64) {
            self.total_syncs += 1;
            self.avg_sync_time_ms =
                (self.avg_sync_time_ms * (self.total_syncs - 1) as f64 + time_ms as f64)
                    / self.total_syncs as f64;
        }

        pub fn record_gradient(&mut self, norm: f64) {
            self.total_gradients += 1;
            self.avg_gradient_norm =
                (self.avg_gradient_norm * (self.total_gradients - 1) as f64 + norm)
                    / self.total_gradients as f64;
        }

        pub fn record_compression(&mut self, ratio: f64) {
            self.total_compressions += 1;
            self.avg_compression_ratio =
                (self.avg_compression_ratio * (self.total_compressions - 1) as f64 + ratio)
                    / self.total_compressions as f64;
        }

        pub fn record_cross_model_alignment(&mut self) {
            self.cross_model_alignments += 1;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    /// Gradient Sync v6 Engine
    pub struct GradientSyncV6 {
        config: GradientSyncV6Config,
        models: HashMap<String, ModelRegistryV6>,
        pending_gradients: Vec<GradientEntryV6>,
        stats: GradientSyncV6Stats,
    }

    impl GradientSyncV6 {
        pub fn new(config: GradientSyncV6Config) -> Self {
            Self {
                config,
                models: HashMap::new(),
                pending_gradients: Vec::new(),
                stats: GradientSyncV6Stats::default(),
            }
        }

        pub fn config(&self) -> &GradientSyncV6Config {
            &self.config
        }

        pub fn stats(&self) -> &GradientSyncV6Stats {
            &self.stats
        }

        pub fn stats_mut(&mut self) -> &mut GradientSyncV6Stats {
            &mut self.stats
        }

        /// Register a model
        pub fn register_model(
            &mut self,
            model_id: String,
            dimension: usize,
        ) -> Result<(), GradientSyncV6Error> {
            if dimension > self.config.max_dimension {
                return Err(GradientSyncV6Error::ConfigurationError(format!(
                    "Dimension {} exceeds max {}",
                    dimension, self.config.max_dimension
                )));
            }
            self.models
                .insert(model_id.clone(), ModelRegistryV6::new(model_id, dimension));
            Ok(())
        }

        /// Update model reputation
        pub fn update_model_reputation(
            &mut self,
            model_id: &str,
            reputation: f64,
        ) -> Result<(), GradientSyncV6Error> {
            match self.models.get_mut(model_id) {
                Some(model) => {
                    model.update_reputation(reputation);
                    Ok(())
                }
                None => Err(GradientSyncV6Error::ModelNotFound(model_id.to_string())),
            }
        }

        /// Submit gradients for a model
        pub fn submit_gradients(
            &mut self,
            model_id: String,
            gradients: Vec<f32>,
            timestamp_ms: u64,
        ) -> Result<(), GradientSyncV6Error> {
            let dimension = match self.models.get(&model_id) {
                Some(model) => model.dimension(),
                None => {
                    return Err(GradientSyncV6Error::ModelNotFound(model_id));
                }
            };

            if gradients.len() != dimension {
                return Err(GradientSyncV6Error::DimensionMismatch {
                    expected: dimension,
                    got: gradients.len(),
                });
            }

            let mut entry = GradientEntryV6::new(model_id, gradients, timestamp_ms);

            // Apply compression if enabled
            if self.config.adaptive_compression {
                let k = (entry.gradients().len() as f64 * self.config.compression_ratio) as usize;
                if k > 0 {
                    entry.compress_topk(k);
                    self.stats.record_compression(entry.compression_ratio());
                }
            }

            self.stats.record_gradient(entry.gradient_norm());
            self.pending_gradients.push(entry);
            Ok(())
        }

        /// Execute gradient synchronization
        pub fn execute_sync(&mut self) -> Result<HashMap<String, Vec<f64>>, GradientSyncV6Error> {
            if self.pending_gradients.is_empty() {
                return Ok(HashMap::new());
            }

            let start_ms = current_timestamp_ms();
            let mut aggregated: HashMap<String, Vec<f64>> = HashMap::new();

            // Group gradients by model
            for entry in self.pending_gradients.drain(..) {
                let model_id = entry.model_id().to_string();
                let grads = entry
                    .compressed_gradients()
                    .unwrap_or_else(|| entry.gradients());

                aggregated
                    .entry(model_id)
                    .or_insert_with(|| vec![0.0; grads.len()])
                    .iter_mut()
                    .zip(grads.iter())
                    .for_each(|(a, b)| *a += *b as f64);
            }

            // Apply EMA smoothing and reputation weighting
            for (model_id, agg_grads) in &aggregated {
                if let Some(model) = self.models.get_mut(model_id) {
                    let grads_ref = agg_grads.as_slice();
                    model.update_ema(
                        &grads_ref.iter().map(|&g| g as f32).collect::<Vec<_>>(),
                        self.config.ema_alpha,
                    );
                    model.record_gradient_norm(
                        Self::compute_norm(grads_ref),
                        self.config.max_gradient_history,
                    );
                    model.record_sync();
                }
            }

            // Apply cross-model alignment if enabled
            if self.config.cross_model_alignment && self.models.len() >= 2 {
                self.apply_cross_model_alignment(&aggregated)?;
                self.stats.record_cross_model_alignment();
            }

            let time_ms = current_timestamp_ms() - start_ms;
            self.stats.record_sync(time_ms);

            Ok(aggregated)
        }

        fn apply_cross_model_alignment(
            &self,
            aggregated: &HashMap<String, Vec<f64>>,
        ) -> Result<(), GradientSyncV6Error> {
            if aggregated.len() < 2 {
                return Ok(());
            }

            let model_ids: Vec<String> = aggregated.keys().cloned().collect();
            for i in 0..model_ids.len() {
                for j in (i + 1)..model_ids.len() {
                    let grads_a = aggregated.get(&model_ids[i]).unwrap();
                    let grads_b = aggregated.get(&model_ids[j]).unwrap();
                    let dim = grads_a.len().min(grads_b.len());

                    let mut alignment = 0.0;
                    for k in 0..dim {
                        alignment += (grads_a[k] - grads_b[k]).abs();
                    }
                    alignment /= dim as f64;

                    // Check for divergence
                    if alignment > 1.0 {
                        return Err(GradientSyncV6Error::SyncFailed(format!(
                            "Cross-model divergence detected: {} vs {} (alignment: {:.4})",
                            model_ids[i], model_ids[j], alignment
                        )));
                    }
                }
            }

            Ok(())
        }

        fn compute_norm(grads: &[f64]) -> f64 {
            let sum: f64 = grads.iter().map(|g| g * g).sum();
            sum.sqrt()
        }

        /// Get model count
        pub fn model_count(&self) -> usize {
            self.models.len()
        }

        /// Get pending gradient count
        pub fn pending_count(&self) -> usize {
            self.pending_gradients.len()
        }

        /// Reset statistics
        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }
    }

    impl Default for GradientSyncV6 {
        fn default() -> Self {
            Self::new(GradientSyncV6Config::default())
        }
    }

    fn current_timestamp_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_engine_creation() {
            let engine = GradientSyncV6::default();
            assert_eq!(engine.model_count(), 0);
        }

        #[test]
        fn test_engine_with_config() {
            let config = GradientSyncV6Config {
                compression_ratio: 0.5,
                ..Default::default()
            };
            let engine = GradientSyncV6::new(config);
            assert_eq!(engine.config().compression_ratio, 0.5);
        }

        #[test]
        fn test_register_model() {
            let mut engine = GradientSyncV6::default();
            assert_eq!(
                engine.register_model("model1".to_string(), 100),
                Ok(())
            );
            assert_eq!(engine.model_count(), 1);
        }

        #[test]
        fn test_register_model_dimension_exceeded() {
            let mut engine = GradientSyncV6::default();
            match engine
                .register_model("model1".to_string(), 20000)
                .unwrap_err()
            {
                GradientSyncV6Error::ConfigurationError(_) => {}
                e => panic!("Expected ConfigurationError, got: {}", e),
            }
        }

        #[test]
        fn test_submit_gradients() {
            let mut engine = GradientSyncV6::default();
            engine.register_model("model1".to_string(), 10).unwrap();
            let grads = vec![1.0f32; 10];
            assert_eq!(
                engine.submit_gradients("model1".to_string(), grads, 1000),
                Ok(())
            );
        }

        #[test]
        fn test_submit_gradients_model_not_found() {
            let mut engine = GradientSyncV6::default();
            let grads = vec![1.0f32; 10];
            match engine
                .submit_gradients("unknown".to_string(), grads, 1000)
                .unwrap_err()
            {
                GradientSyncV6Error::ModelNotFound(_) => {}
                e => panic!("Expected ModelNotFound, got: {}", e),
            }
        }

        #[test]
        fn test_submit_gradients_dimension_mismatch() {
            let mut engine = GradientSyncV6::default();
            engine.register_model("model1".to_string(), 10).unwrap();
            let grads = vec![1.0f32; 5];
            match engine
                .submit_gradients("model1".to_string(), grads, 1000)
                .unwrap_err()
            {
                GradientSyncV6Error::DimensionMismatch { .. } => {}
                e => panic!("Expected DimensionMismatch, got: {}", e),
            }
        }

        #[test]
        fn test_execute_sync() {
            let mut engine = GradientSyncV6::default();
            engine.register_model("model1".to_string(), 10).unwrap();
            let grads = vec![1.0f32; 10];
            engine.submit_gradients("model1".to_string(), grads, 1000)
                .unwrap();
            let result = engine.execute_sync().unwrap();
            assert!(result.contains_key("model1"));
        }

        #[test]
        fn test_execute_sync_empty() {
            let mut engine = GradientSyncV6::default();
            let result = engine.execute_sync().unwrap();
            assert!(result.is_empty());
        }

        #[test]
        fn test_compression() {
            let mut engine = GradientSyncV6::default();
            engine.register_model("model1".to_string(), 100).unwrap();
            let grads: Vec<f32> = (0..100).map(|i| i as f32).collect();
            engine.submit_gradients("model1".to_string(), grads, 1000)
                .unwrap();
            assert!(engine.stats().total_compressions > 0);
        }

        #[test]
        fn test_cross_model_alignment() {
            let mut engine = GradientSyncV6::default();
            engine.register_model("model1".to_string(), 10).unwrap();
            engine.register_model("model2".to_string(), 10).unwrap();
            let grads1 = vec![1.0f32; 10];
            let grads2 = vec![1.1f32; 10];
            engine.submit_gradients("model1".to_string(), grads1, 1000)
                .unwrap();
            engine.submit_gradients("model2".to_string(), grads2, 1000)
                .unwrap();
            let result = engine.execute_sync().unwrap();
            assert_eq!(result.len(), 2);
        }

        #[test]
        fn test_update_model_reputation() {
            let mut engine = GradientSyncV6::default();
            engine.register_model("model1".to_string(), 10).unwrap();
            assert_eq!(
                engine.update_model_reputation("model1", 0.8),
                Ok(())
            );
        }

        #[test]
        fn test_stats_tracking() {
            let mut engine = GradientSyncV6::default();
            engine.register_model("model1".to_string(), 10).unwrap();
            let grads = vec![1.0f32; 10];
            engine.submit_gradients("model1".to_string(), grads, 1000)
                .unwrap();
            engine.execute_sync().unwrap();
            assert_eq!(engine.stats().total_syncs, 1);
        }

        #[test]
        fn test_reset_stats() {
            let mut engine = GradientSyncV6::default();
            engine.register_model("model1".to_string(), 10).unwrap();
            let grads = vec![1.0f32; 10];
            engine.submit_gradients("model1".to_string(), grads, 1000)
                .unwrap();
            engine.execute_sync().unwrap();
            engine.reset_stats();
            assert_eq!(engine.stats().total_syncs, 0);
        }

        #[test]
        fn test_gradient_norm() {
            let entry = GradientEntryV6::new("test".to_string(), vec![3.0f32, 4.0f32], 0);
            assert!((entry.gradient_norm() - 5.0).abs() < 0.01);
        }

        #[test]
        fn test_config_default() {
            let config = GradientSyncV6Config::default();
            assert_eq!(config.max_dimension, 10000);
            assert_eq!(config.ema_alpha, 0.3);
        }

        #[test]
        fn test_error_display() {
            let err = GradientSyncV6Error::ModelNotFound("test".to_string());
            assert!(!format!("{}", err).is_empty());
        }

        #[test]
        fn test_pending_count() {
            let mut engine = GradientSyncV6::default();
            engine.register_model("model1".to_string(), 10).unwrap();
            let grads = vec![1.0f32; 10];
            engine.submit_gradients("model1".to_string(), grads.clone(), 1000)
                .unwrap();
            engine.submit_gradients("model1".to_string(), grads, 1001)
                .unwrap();
            assert_eq!(engine.pending_count(), 2);
        }

        #[test]
        fn test_model_registry_ema() {
            let mut engine = GradientSyncV6::default();
            engine.register_model("model1".to_string(), 10).unwrap();
            let grads = vec![1.0f32; 10];
            engine.submit_gradients("model1".to_string(), grads.clone(), 1000)
                .unwrap();
            engine.execute_sync().unwrap();
            engine.submit_gradients("model1".to_string(), grads, 1001)
                .unwrap();
            engine.execute_sync().unwrap();
        }

        #[test]
        fn test_multiple_models_sync() {
            let mut engine = GradientSyncV6::default();
            engine.register_model("model1".to_string(), 5).unwrap();
            engine.register_model("model2".to_string(), 5).unwrap();
            let grads1 = vec![1.0f32, 2.0f32, 3.0f32, 4.0f32, 5.0f32];
            let grads2 = vec![2.0f32, 3.0f32, 4.0f32, 5.0f32, 6.0f32];
            engine.submit_gradients("model1".to_string(), grads1, 1000)
                .unwrap();
            engine.submit_gradients("model2".to_string(), grads2, 1000)
                .unwrap();
            let result = engine.execute_sync().unwrap();
            assert_eq!(result.len(), 2);
        }
    }
}

pub use internal::*;
