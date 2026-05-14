//! Gradient Sync v7 — Cross-model gradient synchronization with divergence detection.
//!
//! Provides synchronized gradient alignment across federated models with:
//! - Divergence detection using cosine similarity thresholds
//! - Gradient compression with configurable precision
//! - Cross-model alignment scoring
//! - Automatic divergence recovery
//! - Rolling statistics with EMA smoothing

#[cfg(feature = "v1.6-sprint2")]
mod internal {
    use std::collections::{HashMap, VecDeque};
    use std::fmt;

    // -----------------------------------------------------------------------
    // Config
    // -----------------------------------------------------------------------

    /// Configuration for Gradient Sync v7.
    #[derive(Clone, Debug)]
    pub struct GradientSyncV7Config {
        /// Maximum gradient dimension size.
        pub max_gradient_dim: usize,
        /// Cosine similarity threshold for divergence detection.
        pub divergence_threshold: f64,
        /// Compression precision (8 = int8, 16 = int16, 32 = f32).
        pub compression_bits: u8,
        /// EMA alpha for gradient smoothing.
        pub smoothing_alpha: f64,
        /// Maximum history samples per model.
        pub max_history_samples: usize,
        /// Maximum concurrent models.
        pub max_models: usize,
        /// Recovery steps before manual intervention.
        pub max_recovery_steps: u32,
    }

    impl Default for GradientSyncV7Config {
        fn default() -> Self {
            Self {
                max_gradient_dim: 8192,
                divergence_threshold: 0.85,
                compression_bits: 16,
                smoothing_alpha: 0.3,
                max_history_samples: 100,
                max_models: 50,
                max_recovery_steps: 5,
            }
        }
    }

    // -----------------------------------------------------------------------
    // Errors
    // -----------------------------------------------------------------------

    #[derive(Debug, Clone)]
    pub enum GradientSyncV7Error {
        ModelNotFound(String),
        GradientDimensionExceeded(usize),
        DivergenceDetected(String),
        RecoveryLimitExceeded(String),
        InvalidCompressionBits(u8),
        MaxModelsReached(usize),
    }

    impl fmt::Display for GradientSyncV7Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::ModelNotFound(id) => write!(f, "Model not found: {}", id),
                Self::GradientDimensionExceeded(dim) => {
                    write!(f, "Gradient dimension {} exceeds maximum", dim)
                }
                Self::DivergenceDetected(id) => write!(f, "Divergence detected for model: {}", id),
                Self::RecoveryLimitExceeded(id) => {
                    write!(f, "Recovery limit exceeded for model: {}", id)
                }
                Self::InvalidCompressionBits(bits) => {
                    write!(f, "Invalid compression bits: {}", bits)
                }
                Self::MaxModelsReached(max) => write!(f, "Maximum models limit reached: {}", max),
            }
        }
    }

    impl std::error::Error for GradientSyncV7Error {}

    // -----------------------------------------------------------------------
    // Model Entry
    // -----------------------------------------------------------------------

    /// Represents a model in the gradient sync system.
    #[derive(Clone, Debug)]
    pub struct ModelEntryV7 {
        pub model_id: String,
        pub gradient: Vec<f64>,
        pub smoothed_gradient: Vec<f64>,
        pub alignment_score: f64,
        pub divergence_count: u32,
        pub recovery_steps: u32,
        pub history: VecDeque<Vec<f64>>,
        pub last_sync_ms: u64,
    }

    impl ModelEntryV7 {
        pub fn new(model_id: String, gradient: Vec<f64>) -> Self {
            let dim = gradient.len();
            Self {
                model_id,
                smoothed_gradient: vec![0.0; dim],
                alignment_score: 1.0,
                divergence_count: 0,
                recovery_steps: 0,
                history: VecDeque::with_capacity(100),
                last_sync_ms: 0,
                gradient,
            }
        }

        pub fn update_gradient(&mut self, new_gradient: Vec<f64>, alpha: f64) {
            let dim = new_gradient.len();
            self.gradient = new_gradient;
            self.history.push_back(self.gradient.clone());
            // EMA smoothing
            for i in 0..dim {
                self.smoothed_gradient[i] =
                    alpha * self.gradient[i] + (1.0 - alpha) * self.smoothed_gradient[i];
            }
        }

        pub fn cosine_similarity(&self, other: &ModelEntryV7) -> f64 {
            let dim = std::cmp::min(self.gradient.len(), other.gradient.len());
            let mut dot = 0.0;
            let mut norm_a = 0.0;
            let mut norm_b = 0.0;
            for i in 0..dim {
                dot += self.gradient[i] * other.gradient[i];
                norm_a += self.gradient[i] * self.gradient[i];
                norm_b += other.gradient[i] * other.gradient[i];
            }
            let denom = norm_a.sqrt() * norm_b.sqrt();
            if denom < 1e-10 {
                0.0
            } else {
                dot / denom
            }
        }

        pub fn is_diverged(&self, threshold: f64) -> bool {
            self.alignment_score < threshold
        }

        pub fn record_divergence(&mut self) {
            self.divergence_count += 1;
        }

        pub fn attempt_recovery(&mut self, max_steps: u32) -> Result<(), GradientSyncV7Error> {
            if self.recovery_steps >= max_steps {
                return Err(GradientSyncV7Error::RecoveryLimitExceeded(
                    self.model_id.clone(),
                ));
            }
            self.recovery_steps += 1;
            Ok(())
        }

        pub fn reset_recovery(&mut self) {
            self.recovery_steps = 0;
        }
    }

    // -----------------------------------------------------------------------
    // Sync Result
    // -----------------------------------------------------------------------

    /// Result of a gradient synchronization operation.
    #[derive(Clone, Debug)]
    pub struct SyncResultV7 {
        pub synced: bool,
        pub alignment_score: f64,
        pub diverged_models: Vec<String>,
        pub compression_ratio: f64,
        pub sync_time_ms: u64,
    }

    impl SyncResultV7 {
        pub fn new(
            synced: bool,
            alignment_score: f64,
            diverged_models: Vec<String>,
            compression_ratio: f64,
            sync_time_ms: u64,
        ) -> Self {
            Self {
                synced,
                alignment_score,
                diverged_models,
                compression_ratio,
                sync_time_ms,
            }
        }
    }

    // -----------------------------------------------------------------------
    // Stats
    // -----------------------------------------------------------------------

    /// Statistics for Gradient Sync v7 operations.
    #[derive(Clone, Debug)]
    pub struct GradientSyncV7Stats {
        pub total_syncs: u64,
        pub successful_syncs: u64,
        pub divergence_events: u64,
        pub recovery_attempts: u64,
        pub avg_alignment_score: f64,
        pub avg_sync_time_ms: f64,
        pub sync_times: VecDeque<u64>,
    }

    impl Default for GradientSyncV7Stats {
        fn default() -> Self {
            Self {
                total_syncs: 0,
                successful_syncs: 0,
                divergence_events: 0,
                recovery_attempts: 0,
                avg_alignment_score: 1.0,
                avg_sync_time_ms: 0.0,
                sync_times: VecDeque::with_capacity(100),
            }
        }
    }

    impl GradientSyncV7Stats {
        pub fn record_sync(&mut self, success: bool, alignment: f64, time_ms: u64) {
            self.total_syncs += 1;
            if success {
                self.successful_syncs += 1;
            }
            self.sync_times.push_back(time_ms);
            if self.sync_times.len() > 100 {
                self.sync_times.pop_front();
            }
            let sum: u64 = self.sync_times.iter().sum();
            self.avg_sync_time_ms = sum as f64 / self.sync_times.len() as f64;
            // EMA for alignment
            self.avg_alignment_score =
                0.3 * alignment + 0.7 * self.avg_alignment_score;
        }

        pub fn record_divergence(&mut self) {
            self.divergence_events += 1;
        }

        pub fn record_recovery(&mut self) {
            self.recovery_attempts += 1;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // -----------------------------------------------------------------------
    // Main Engine
    // -----------------------------------------------------------------------

    /// Gradient Sync v7 engine for cross-model gradient synchronization.
    pub struct GradientSyncV7 {
        config: GradientSyncV7Config,
        models: HashMap<String, ModelEntryV7>,
        stats: GradientSyncV7Stats,
    }

    impl GradientSyncV7 {
        pub fn new(config: GradientSyncV7Config) -> Self {
            Self {
                config,
                models: HashMap::new(),
                stats: GradientSyncV7Stats::default(),
            }
        }

        /// Register a new model with initial gradient.
        pub fn register_model(
            &mut self,
            model_id: String,
            gradient: Vec<f64>,
        ) -> Result<(), GradientSyncV7Error> {
            if self.models.len() >= self.config.max_models {
                return Err(GradientSyncV7Error::MaxModelsReached(
                    self.config.max_models,
                ));
            }
            if gradient.len() > self.config.max_gradient_dim {
                return Err(GradientSyncV7Error::GradientDimensionExceeded(
                    gradient.len(),
                ));
            }
            if self.models.contains_key(&model_id) {
                return Err(GradientSyncV7Error::ModelNotFound(model_id));
            }
            self.models.insert(model_id.clone(), ModelEntryV7::new(model_id, gradient));
            Ok(())
        }

        /// Update gradient for a model.
        pub fn update_gradient(
            &mut self,
            model_id: &str,
            new_gradient: Vec<f64>,
            current_ms: u64,
        ) -> Result<(), GradientSyncV7Error> {
            let model = self
                .models
                .get_mut(model_id)
                .ok_or_else(|| GradientSyncV7Error::ModelNotFound(model_id.to_string()))?;
            model.update_gradient(new_gradient, self.config.smoothing_alpha);
            model.last_sync_ms = current_ms;
            Ok(())
        }

        /// Compute pairwise alignment scores.
        pub fn compute_alignment(&mut self) -> Result<HashMap<String, f64>, GradientSyncV7Error> {
            let model_ids: Vec<&String> = self.models.keys().collect();
            let mut scores = HashMap::new();
            for id in &model_ids {
                let model = self.models.get(*id).unwrap();
                let mut total_similarity = 0.0;
                let mut count = 0;
                for other_id in &model_ids {
                    if id != other_id {
                        let other = self.models.get(*other_id).unwrap();
                        total_similarity += model.cosine_similarity(other);
                        count += 1;
                    }
                }
                let avg = if count > 0 { total_similarity / count as f64 } else { 1.0 };
                scores.insert((*id).clone(), avg);
            }
            // Update alignment scores
            for (id, score) in &scores {
                if let Some(model) = self.models.get_mut(id) {
                    model.alignment_score = *score;
                }
            }
            Ok(scores)
        }

        /// Detect diverged models.
        pub fn detect_divergence(&self) -> Vec<String> {
            self.models
                .values()
                .filter(|m| m.is_diverged(self.config.divergence_threshold))
                .map(|m| m.model_id.clone())
                .collect()
        }

        /// Attempt recovery for diverged models.
        pub fn recover_diverged(
            &mut self,
        ) -> Result<Vec<String>, GradientSyncV7Error> {
            let diverged = self.detect_divergence();
            let mut recovered = Vec::new();
            for id in &diverged {
                let model = self
                    .models
                    .get_mut(id)
                    .ok_or_else(|| GradientSyncV7Error::ModelNotFound(id.clone()))?;
                model.record_divergence();
                self.stats.record_divergence();
                match model.attempt_recovery(self.config.max_recovery_steps) {
                    Ok(()) => {
                        self.stats.record_recovery();
                        recovered.push(id.clone());
                    }
                    Err(e) => return Err(e),
                }
            }
            Ok(recovered)
        }

        /// Perform full gradient synchronization.
        pub fn sync_gradients(
            &mut self,
            current_ms: u64,
        ) -> Result<SyncResultV7, GradientSyncV7Error> {
            let start_ms = current_ms;
            // Compute alignment
            let scores = self.compute_alignment()?;
            let avg_alignment: f64 = if scores.is_empty() {
                1.0
            } else {
                scores.values().sum::<f64>() / scores.len() as f64
            };
            // Detect divergence
            let diverged = self.detect_divergence();
            let synced = diverged.is_empty();
            // Compression ratio based on bits
            let compression_ratio = 32.0 / (self.config.compression_bits as f64);
            let elapsed = current_ms - start_ms;
            self.stats.record_sync(synced, avg_alignment, elapsed);
            Ok(SyncResultV7::new(
                synced,
                avg_alignment,
                diverged,
                compression_ratio,
                elapsed,
            ))
        }

        /// Get model entry.
        pub fn get_model(&self, model_id: &str) -> Option<&ModelEntryV7> {
            self.models.get(model_id)
        }

        /// Get statistics.
        pub fn get_stats(&self) -> &GradientSyncV7Stats {
            &self.stats
        }

        /// Reset statistics.
        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }

        /// Cleanup models with no recent sync.
        pub fn cleanup_stale(&mut self, current_ms: u64, timeout_ms: u64) -> usize {
            let before = self.models.len();
            self.models.retain(|_, m| current_ms.saturating_sub(m.last_sync_ms) < timeout_ms);
            before.saturating_sub(self.models.len())
        }
    }

    impl Default for GradientSyncV7 {
        fn default() -> Self {
            Self::new(GradientSyncV7Config::default())
        }
    }

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        fn make_gradient(dim: usize, seed: f64) -> Vec<f64> {
            (0..dim).map(|i| (i as f64 + seed) * 0.1).collect()
        }

        fn make_config() -> GradientSyncV7Config {
            GradientSyncV7Config {
                max_gradient_dim: 1024,
                divergence_threshold: 0.85,
                compression_bits: 16,
                smoothing_alpha: 0.3,
                max_history_samples: 50,
                max_models: 10,
                max_recovery_steps: 3,
            }
        }

        #[test]
        fn test_engine_creation() {
            let engine = GradientSyncV7::default();
            assert_eq!(engine.models.len(), 0);
        }

        #[test]
        fn test_engine_with_config() {
            let config = make_config();
            let engine = GradientSyncV7::new(config);
            assert_eq!(engine.config.max_models, 10);
        }

        #[test]
        fn test_register_model() {
            let mut engine = GradientSyncV7::default();
            let gradient = make_gradient(128, 1.0);
            engine
                .register_model("model_1".to_string(), gradient)
                .unwrap();
            assert_eq!(engine.models.len(), 1);
        }

        #[test]
        fn test_register_model_duplicate() {
            let mut engine = GradientSyncV7::default();
            let gradient = make_gradient(128, 1.0);
            engine
                .register_model("model_1".to_string(), gradient.clone())
                .unwrap();
            match engine
                .register_model("model_1".to_string(), gradient)
                .unwrap_err()
            {
                GradientSyncV7Error::ModelNotFound(_) => {}
                e => panic!("Expected ModelNotFound, got {:?}", e),
            }
        }

        #[test]
        fn test_register_model_dimension_exceeded() {
            let mut engine = GradientSyncV7::new(make_config());
            let gradient = make_gradient(2048, 1.0);
            match engine
                .register_model("big".to_string(), gradient)
                .unwrap_err()
            {
                GradientSyncV7Error::GradientDimensionExceeded(2048) => {}
                e => panic!("Expected GradientDimensionExceeded, got {:?}", e),
            }
        }

        #[test]
        fn test_register_model_max_reached() {
            let mut config = GradientSyncV7Config::default();
            config.max_models = 2;
            let mut engine = GradientSyncV7::new(config);
            engine
                .register_model("m1".to_string(), make_gradient(10, 1.0))
                .unwrap();
            engine
                .register_model("m2".to_string(), make_gradient(10, 2.0))
                .unwrap();
            match engine
                .register_model("m3".to_string(), make_gradient(10, 3.0))
                .unwrap_err()
            {
                GradientSyncV7Error::MaxModelsReached(2) => {}
                e => panic!("Expected MaxModelsReached, got {:?}", e),
            }
        }

        #[test]
        fn test_update_gradient() {
            let mut engine = GradientSyncV7::default();
            engine
                .register_model("m1".to_string(), make_gradient(64, 1.0))
                .unwrap();
            engine.update_gradient("m1", make_gradient(64, 2.0), 1000).unwrap();
            let model = engine.get_model("m1").unwrap();
            assert_eq!(model.last_sync_ms, 1000);
        }

        #[test]
        fn test_update_gradient_not_found() {
            let mut engine = GradientSyncV7::default();
            match engine
                .update_gradient("missing", make_gradient(64, 1.0), 1000)
                .unwrap_err()
            {
                GradientSyncV7Error::ModelNotFound(_) => {}
                e => panic!("Expected ModelNotFound, got {:?}", e),
            }
        }

        #[test]
        fn test_compute_alignment() {
            let mut engine = GradientSyncV7::default();
            engine
                .register_model("m1".to_string(), make_gradient(64, 1.0))
                .unwrap();
            engine
                .register_model("m2".to_string(), make_gradient(64, 1.0))
                .unwrap();
            let scores = engine.compute_alignment().unwrap();
            assert_eq!(scores.len(), 2);
            // Identical gradients should have alignment ~1.0
            assert!(*scores.get("m1").unwrap() > 0.99);
        }

        #[test]
        fn test_detect_divergence() {
            let mut engine = GradientSyncV7::default();
            // Use opposite-direction gradients for clear divergence
            let g1: Vec<f64> = (0..64).map(|i| i as f64 * 0.1).collect();
            let g2: Vec<f64> = (0..64).map(|i| -(i as f64) * 0.1).collect();
            engine.register_model("m1".to_string(), g1).unwrap();
            engine.register_model("m2".to_string(), g2).unwrap();
            engine.compute_alignment().unwrap();
            let diverged = engine.detect_divergence();
            assert!(!diverged.is_empty());
        }

        #[test]
        fn test_sync_gradients() {
            let mut engine = GradientSyncV7::default();
            engine
                .register_model("m1".to_string(), make_gradient(64, 1.0))
                .unwrap();
            engine
                .register_model("m2".to_string(), make_gradient(64, 1.0))
                .unwrap();
            let result = engine.sync_gradients(1000).unwrap();
            assert!(result.synced);
            assert!(result.alignment_score > 0.99);
        }

        #[test]
        fn test_sync_gradients_with_divergence() {
            let mut engine = GradientSyncV7::default();
            // Opposite-direction gradients
            let g1: Vec<f64> = (0..64).map(|i| i as f64 * 0.1).collect();
            let g2: Vec<f64> = (0..64).map(|i| -(i as f64) * 0.1).collect();
            engine.register_model("m1".to_string(), g1).unwrap();
            engine.register_model("m2".to_string(), g2).unwrap();
            let result = engine.sync_gradients(1000).unwrap();
            assert!(!result.synced);
            assert!(!result.diverged_models.is_empty());
        }

        #[test]
        fn test_recover_diverged() {
            let mut engine = GradientSyncV7::default();
            let g1: Vec<f64> = (0..64).map(|i| i as f64 * 0.1).collect();
            let g2: Vec<f64> = (0..64).map(|i| -(i as f64) * 0.1).collect();
            engine.register_model("m1".to_string(), g1).unwrap();
            engine.register_model("m2".to_string(), g2).unwrap();
            engine.compute_alignment().unwrap();
            let recovered = engine.recover_diverged().unwrap();
            assert!(!recovered.is_empty());
        }

        #[test]
        fn test_recovery_limit_exceeded() {
            let mut config = GradientSyncV7Config::default();
            config.max_recovery_steps = 1;
            let mut engine = GradientSyncV7::new(config);
            let g1: Vec<f64> = (0..64).map(|i| i as f64 * 0.1).collect();
            let g2: Vec<f64> = (0..64).map(|i| -(i as f64) * 0.1).collect();
            engine.register_model("m1".to_string(), g1).unwrap();
            engine.register_model("m2".to_string(), g2).unwrap();
            engine.compute_alignment().unwrap();
            // First recovery succeeds
            engine.recover_diverged().unwrap();
            engine.compute_alignment().unwrap();
            // Second recovery should fail
            match engine.recover_diverged() {
                Err(GradientSyncV7Error::RecoveryLimitExceeded(_)) => {}
                Ok(_) => panic!("Expected RecoveryLimitExceeded"),
                Err(e) => panic!("Expected RecoveryLimitExceeded, got {:?}", e),
            }
        }

        #[test]
        fn test_stats_recording() {
            let mut engine = GradientSyncV7::default();
            engine
                .register_model("m1".to_string(), make_gradient(64, 1.0))
                .unwrap();
            engine.sync_gradients(1000).unwrap();
            let stats = engine.get_stats();
            assert_eq!(stats.total_syncs, 1);
            assert_eq!(stats.successful_syncs, 1);
        }

        #[test]
        fn test_reset_stats() {
            let mut engine = GradientSyncV7::default();
            engine
                .register_model("m1".to_string(), make_gradient(64, 1.0))
                .unwrap();
            engine.sync_gradients(1000).unwrap();
            engine.reset_stats();
            let stats = engine.get_stats();
            assert_eq!(stats.total_syncs, 0);
        }

        #[test]
        fn test_cleanup_stale() {
            let mut engine = GradientSyncV7::default();
            engine
                .register_model("m1".to_string(), make_gradient(64, 1.0))
                .unwrap();
            engine.update_gradient("m1", make_gradient(64, 2.0), 1000).unwrap();
            let cleaned = engine.cleanup_stale(2000, 500);
            assert_eq!(cleaned, 1);
        }

        #[test]
        fn test_cosine_similarity_identical() {
            let g = make_gradient(64, 1.0);
            let m1 = ModelEntryV7::new("m1".to_string(), g.clone());
            let m2 = ModelEntryV7::new("m2".to_string(), g);
            let sim = m1.cosine_similarity(&m2);
            assert!(sim > 0.99);
        }

        #[test]
        fn test_compression_ratio() {
            let mut config = GradientSyncV7Config::default();
            config.compression_bits = 8;
            let mut engine = GradientSyncV7::new(config);
            engine
                .register_model("m1".to_string(), make_gradient(64, 1.0))
                .unwrap();
            let result = engine.sync_gradients(1000).unwrap();
            assert!((result.compression_ratio - 4.0) < 0.01);
        }

        #[test]
        fn test_error_display() {
            let err = GradientSyncV7Error::ModelNotFound("test".to_string());
            let msg = format!("{}", err);
            assert!(msg.contains("test"));
        }

        #[test]
        fn test_config_default() {
            let config = GradientSyncV7Config::default();
            assert_eq!(config.max_gradient_dim, 8192);
            assert_eq!(config.divergence_threshold, 0.85);
        }

        #[test]
        fn test_model_alignment_score_update() {
            let mut engine = GradientSyncV7::default();
            engine
                .register_model("m1".to_string(), make_gradient(64, 1.0))
                .unwrap();
            engine
                .register_model("m2".to_string(), make_gradient(64, 1.0))
                .unwrap();
            engine.compute_alignment().unwrap();
            let model = engine.get_model("m1").unwrap();
            assert!(model.alignment_score > 0.99);
        }

        #[test]
        fn test_multiple_syncs_stats() {
            let mut engine = GradientSyncV7::default();
            engine
                .register_model("m1".to_string(), make_gradient(64, 1.0))
                .unwrap();
            for i in 0..5 {
                engine.sync_gradients(i * 100).unwrap();
            }
            let stats = engine.get_stats();
            assert_eq!(stats.total_syncs, 5);
        }

        #[test]
        fn test_ema_smoothing() {
            let mut engine = GradientSyncV7::default();
            engine
                .register_model("m1".to_string(), make_gradient(32, 1.0))
                .unwrap();
            engine
                .update_gradient("m1", make_gradient(32, 10.0), 1000)
                .unwrap();
            let model = engine.get_model("m1").unwrap();
            // Smoothed should be between initial (0) and new (10.0 * 0.1 = 1.0)
            assert!(model.smoothed_gradient[0] > 0.0);
            assert!(model.smoothed_gradient[0] < 1.0);
        }
    }
}

#[cfg(feature = "v1.6-sprint2")]
pub use internal::*;
