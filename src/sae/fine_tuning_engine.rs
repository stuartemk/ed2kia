//! SAE Fine-Tuning Engine — Motor de fine-tuning distribuido para modelos SAE
//!
//! Coordina el fine-tuning distribuido de Sparse Autoencoders con scheduling
//! de learning rate, gestión de estado de entrenamiento y checkpoints.

use std::fmt;
use std::time::Instant;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum FineTuningError {
    TrainingAlreadyStarted,
    TrainingNotStarted,
    EpochAlreadyCompleted,
    InvalidConfig(String),
    CheckpointCorrupted(String),
    ConvergenceFailed(String),
}

impl fmt::Display for FineTuningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FineTuningError::TrainingAlreadyStarted => write!(f, "Training already started"),
            FineTuningError::TrainingNotStarted => write!(f, "Training not started"),
            FineTuningError::EpochAlreadyCompleted => write!(f, "Epoch already completed"),
            FineTuningError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
            FineTuningError::CheckpointCorrupted(msg) => write!(f, "Checkpoint corrupted: {}", msg),
            FineTuningError::ConvergenceFailed(msg) => write!(f, "Convergence failed: {}", msg),
        }
    }
}

impl std::error::Error for FineTuningError {}

// ============================================================================
// Learning Rate Schedule
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum LearningRateSchedule {
    Constant,
    CosineDecay { max_steps: usize },
    StepDecay { step_size: usize, decay_factor: f32 },
}

impl fmt::Display for LearningRateSchedule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LearningRateSchedule::Constant => write!(f, "Constant"),
            LearningRateSchedule::CosineDecay { max_steps } => {
                write!(f, "CosineDecay(max_steps={})", max_steps)
            }
            LearningRateSchedule::StepDecay {
                step_size,
                decay_factor,
            } => write!(f, "StepDecay(step={}, factor={})", step_size, decay_factor),
        }
    }
}

// ============================================================================
// Fine-Tuning Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct FineTuningConfig {
    pub learning_rate: f32,
    pub schedule: LearningRateSchedule,
    pub batch_size: usize,
    pub max_epochs: usize,
    pub convergence_threshold: f32,
}

impl FineTuningConfig {
    pub fn new(learning_rate: f32, batch_size: usize, max_epochs: usize) -> Self {
        Self {
            learning_rate,
            schedule: LearningRateSchedule::Constant,
            batch_size,
            max_epochs,
            convergence_threshold: 0.001,
        }
    }

    pub fn with_schedule(mut self, schedule: LearningRateSchedule) -> Self {
        self.schedule = schedule;
        self
    }

    pub fn with_convergence_threshold(mut self, threshold: f32) -> Self {
        self.convergence_threshold = threshold;
        self
    }

    pub fn validate(&self) -> Result<(), FineTuningError> {
        if self.learning_rate <= 0.0 || self.learning_rate > 1.0 {
            return Err(FineTuningError::InvalidConfig(
                "Learning rate must be between 0 and 1".to_string(),
            ));
        }
        if self.batch_size == 0 {
            return Err(FineTuningError::InvalidConfig(
                "Batch size must be greater than 0".to_string(),
            ));
        }
        if self.max_epochs == 0 {
            return Err(FineTuningError::InvalidConfig(
                "Max epochs must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}

// ============================================================================
// Training State
// ============================================================================

#[derive(Debug, Clone)]
pub struct TrainingState {
    pub epoch: usize,
    pub batch: usize,
    pub current_loss: f32,
    pub best_loss: f32,
    pub is_converged: bool,
    pub total_batches: usize,
    pub started_at: Instant,
}

impl TrainingState {
    pub fn new() -> Self {
        Self {
            epoch: 0,
            batch: 0,
            current_loss: f32::MAX,
            best_loss: f32::MAX,
            is_converged: false,
            total_batches: 0,
            started_at: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }

    pub fn is_complete(&self, max_epochs: usize) -> bool {
        self.is_converged || self.epoch >= max_epochs
    }
}

impl Default for TrainingState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Training Metric
// ============================================================================

#[derive(Debug, Clone)]
pub struct TrainingMetric {
    pub epoch: usize,
    pub batch: usize,
    pub loss: f32,
    pub gradient_norm: f32,
    pub learning_rate: f32,
    pub timestamp: Instant,
}

// ============================================================================
// Training Checkpoint
// ============================================================================

#[derive(Debug, Clone)]
pub struct TrainingCheckpoint {
    pub state: TrainingState,
    pub metrics: Vec<TrainingMetric>,
    pub config: FineTuningConfig,
    pub created_at: Instant,
}

impl TrainingCheckpoint {
    pub fn new(
        state: TrainingState,
        metrics: Vec<TrainingMetric>,
        config: FineTuningConfig,
    ) -> Self {
        Self {
            state,
            metrics,
            config,
            created_at: Instant::now(),
        }
    }
}

// ============================================================================
// Fine-Tuning Engine
// ============================================================================

pub struct FineTuningEngine {
    config: FineTuningConfig,
    state: TrainingState,
    metrics: Vec<TrainingMetric>,
    is_running: bool,
}

impl FineTuningEngine {
    pub fn new(config: FineTuningConfig) -> Self {
        Self {
            config: config.clone(),
            state: TrainingState::new(),
            metrics: Vec::new(),
            is_running: false,
        }
    }

    /// Start a new epoch
    pub fn start_epoch(&mut self) -> Result<(), FineTuningError> {
        if self.state.is_converged {
            return Err(FineTuningError::ConvergenceFailed(
                "Training already converged".to_string(),
            ));
        }
        if self.state.epoch >= self.config.max_epochs {
            return Err(FineTuningError::EpochAlreadyCompleted);
        }

        self.state.epoch += 1;
        self.state.batch = 0;
        self.is_running = true;
        Ok(())
    }

    /// Record a batch of training results
    pub fn record_batch(&mut self, loss: f32, gradient_norm: f32) -> Result<(), FineTuningError> {
        if !self.is_running {
            return Err(FineTuningError::TrainingNotStarted);
        }

        self.state.batch += 1;
        self.state.total_batches += 1;
        self.state.current_loss = loss;

        // Update best loss
        if loss < self.state.best_loss {
            self.state.best_loss = loss;
        }

        // Record metric
        let lr = self.get_learning_rate();
        self.metrics.push(TrainingMetric {
            epoch: self.state.epoch,
            batch: self.state.batch,
            loss,
            gradient_norm,
            learning_rate: lr,
            timestamp: Instant::now(),
        });

        Ok(())
    }

    /// Get the current learning rate based on schedule
    pub fn get_learning_rate(&self) -> f32 {
        match &self.config.schedule {
            LearningRateSchedule::Constant => self.config.learning_rate,
            LearningRateSchedule::CosineDecay { max_steps } => {
                let progress = self.state.total_batches as f32 / (*max_steps as f32).max(1.0);
                let cosine = (std::f32::consts::PI * progress).cos();
                self.config.learning_rate * (0.5 * (1.0 + cosine)).max(0.0)
            }
            LearningRateSchedule::StepDecay {
                step_size,
                decay_factor,
            } => {
                let steps_completed = self.state.total_batches / step_size;
                self.config
                    .learning_rate
                    .powf((steps_completed as f32) * decay_factor.recip().ln().max(0.0))
            }
        }
    }

    /// Check if training has converged
    pub fn check_convergence(&mut self) -> bool {
        if self.metrics.len() < 2 {
            return false;
        }

        // Check if loss has stabilized below threshold
        let recent_losses: Vec<f32> = self.metrics.iter().rev().take(10).map(|m| m.loss).collect();

        if recent_losses.len() < 2 {
            return false;
        }

        let avg_loss: f32 = recent_losses.iter().sum::<f32>() / recent_losses.len() as f32;
        let loss_variance: f32 = recent_losses
            .iter()
            .map(|l| (l - avg_loss).powi(2))
            .sum::<f32>()
            / recent_losses.len() as f32;

        let converged = loss_variance < self.config.convergence_threshold
            || self.state.best_loss < self.config.convergence_threshold;

        if converged {
            self.state.is_converged = true;
        }

        converged
    }

    /// Get current training state
    pub fn get_state(&self) -> &TrainingState {
        &self.state
    }

    /// Get training metrics history
    pub fn get_metrics(&self) -> &[TrainingMetric] {
        &self.metrics
    }

    /// Create a checkpoint of current training state
    pub fn create_checkpoint(&self) -> TrainingCheckpoint {
        TrainingCheckpoint::new(
            self.state.clone(),
            self.metrics.clone(),
            self.config.clone(),
        )
    }

    /// Restore training from a checkpoint
    pub fn restore_checkpoint(&mut self, checkpoint: &TrainingCheckpoint) {
        self.state = checkpoint.state.clone();
        self.metrics = checkpoint.metrics.clone();
        self.config = checkpoint.config.clone();
        self.is_running = true;
    }

    /// Stop training
    pub fn stop(&mut self) {
        self.is_running = false;
    }

    /// Check if training is running
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Get the configuration
    pub fn get_config(&self) -> &FineTuningConfig {
        &self.config
    }

    /// Reset the engine to initial state
    pub fn reset(&mut self) {
        self.state = TrainingState::new();
        self.metrics.clear();
        self.is_running = false;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> FineTuningConfig {
        FineTuningConfig::new(0.001, 32, 10)
    }

    #[test]
    fn test_engine_creation() {
        let config = make_config();
        let engine = FineTuningEngine::new(config);
        assert!(!engine.is_running());
        assert_eq!(engine.get_state().epoch, 0);
    }

    #[test]
    fn test_start_epoch() {
        let config = make_config();
        let mut engine = FineTuningEngine::new(config);
        assert!(engine.start_epoch().is_ok());
        assert_eq!(engine.get_state().epoch, 1);
        assert!(engine.is_running());
    }

    #[test]
    fn test_record_batch() {
        let config = make_config();
        let mut engine = FineTuningEngine::new(config);
        engine.start_epoch().unwrap();
        assert!(engine.record_batch(0.5, 1.0).is_ok());
        assert_eq!(engine.get_state().batch, 1);
        assert_eq!(engine.get_state().current_loss, 0.5);
    }

    #[test]
    fn test_record_batch_without_start() {
        let config = make_config();
        let mut engine = FineTuningEngine::new(config);
        match engine.record_batch(0.5, 1.0) {
            Err(FineTuningError::TrainingNotStarted) => {}
            other => panic!("Expected TrainingNotStarted, got {:?}", other),
        }
    }

    #[test]
    fn test_best_loss_update() {
        let config = make_config();
        let mut engine = FineTuningEngine::new(config);
        engine.start_epoch().unwrap();
        engine.record_batch(0.8, 1.0).unwrap();
        engine.record_batch(0.5, 0.8).unwrap();
        engine.record_batch(0.3, 0.6).unwrap();
        assert_eq!(engine.get_state().best_loss, 0.3);
    }

    #[test]
    fn test_constant_learning_rate() {
        let config = FineTuningConfig::new(0.01, 32, 10);
        let engine = FineTuningEngine::new(config);
        assert_eq!(engine.get_learning_rate(), 0.01);
    }

    #[test]
    fn test_cosine_decay_learning_rate() {
        let config = FineTuningConfig::new(0.01, 32, 10)
            .with_schedule(LearningRateSchedule::CosineDecay { max_steps: 100 });
        let engine = FineTuningEngine::new(config);
        let initial_lr = engine.get_learning_rate();
        assert!(initial_lr > 0.0);
    }

    #[test]
    fn test_step_decay_learning_rate() {
        let config =
            FineTuningConfig::new(0.01, 32, 10).with_schedule(LearningRateSchedule::StepDecay {
                step_size: 10,
                decay_factor: 0.1,
            });
        let engine = FineTuningEngine::new(config);
        let lr = engine.get_learning_rate();
        assert!(lr > 0.0);
    }

    #[test]
    fn test_convergence_check() {
        let config = FineTuningConfig::new(0.001, 32, 10).with_convergence_threshold(0.0001);
        let mut engine = FineTuningEngine::new(config);
        engine.start_epoch().unwrap();
        for _ in 0..20 {
            engine.record_batch(0.0001, 0.001).unwrap();
        }
        assert!(engine.check_convergence());
    }

    #[test]
    fn test_no_convergence_with_high_variance() {
        let config = FineTuningConfig::new(0.001, 32, 10).with_convergence_threshold(0.0001);
        let mut engine = FineTuningEngine::new(config);
        engine.start_epoch().unwrap();
        for i in 0..20 {
            let loss = if i % 2 == 0 { 0.5 } else { 0.1 };
            engine.record_batch(loss, 1.0).unwrap();
        }
        assert!(!engine.check_convergence());
    }

    #[test]
    fn test_create_checkpoint() {
        let config = make_config();
        let mut engine = FineTuningEngine::new(config);
        engine.start_epoch().unwrap();
        engine.record_batch(0.5, 1.0).unwrap();
        let checkpoint = engine.create_checkpoint();
        assert_eq!(checkpoint.state.epoch, 1);
        assert_eq!(checkpoint.metrics.len(), 1);
    }

    #[test]
    fn test_restore_checkpoint() {
        let config = make_config();
        let mut engine = FineTuningEngine::new(config);
        engine.start_epoch().unwrap();
        engine.record_batch(0.5, 1.0).unwrap();
        let checkpoint = engine.create_checkpoint();

        let mut engine2 = FineTuningEngine::new(make_config());
        engine2.restore_checkpoint(&checkpoint);
        assert_eq!(engine2.get_state().epoch, 1);
        assert_eq!(engine2.get_metrics().len(), 1);
    }

    #[test]
    fn test_stop_training() {
        let config = make_config();
        let mut engine = FineTuningEngine::new(config);
        engine.start_epoch().unwrap();
        assert!(engine.is_running());
        engine.stop();
        assert!(!engine.is_running());
    }

    #[test]
    fn test_reset_engine() {
        let config = make_config();
        let mut engine = FineTuningEngine::new(config);
        engine.start_epoch().unwrap();
        engine.record_batch(0.5, 1.0).unwrap();
        engine.reset();
        assert_eq!(engine.get_state().epoch, 0);
        assert_eq!(engine.get_metrics().len(), 0);
        assert!(!engine.is_running());
    }

    #[test]
    fn test_config_validation() {
        let config = FineTuningConfig::new(0.001, 32, 10);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_learning_rate() {
        let config = FineTuningConfig::new(0.0, 32, 10);
        match config.validate() {
            Err(FineTuningError::InvalidConfig(msg)) => {
                assert!(msg.contains("Learning rate"));
            }
            other => panic!("Expected InvalidConfig, got {:?}", other),
        }
    }

    #[test]
    fn test_config_invalid_batch_size() {
        let config = FineTuningConfig::new(0.001, 0, 10);
        match config.validate() {
            Err(FineTuningError::InvalidConfig(msg)) => {
                assert!(msg.contains("Batch size"));
            }
            other => panic!("Expected InvalidConfig, got {:?}", other),
        }
    }

    #[test]
    fn test_multiple_epochs() {
        let config = make_config();
        let mut engine = FineTuningEngine::new(config);
        for epoch in 1..=5 {
            engine.start_epoch().unwrap();
            engine.record_batch(0.5 / epoch as f32, 1.0).unwrap();
            assert_eq!(engine.get_state().epoch, epoch);
        }
    }

    #[test]
    fn test_training_state_elapsed() {
        let config = make_config();
        let engine = FineTuningEngine::new(config);
        let elapsed = engine.get_state().elapsed();
        assert!(elapsed.as_millis() >= 0);
    }

    #[test]
    fn test_schedule_display() {
        let schedule = LearningRateSchedule::CosineDecay { max_steps: 100 };
        assert!(format!("{}", schedule).contains("CosineDecay"));
    }

    #[test]
    fn test_error_display() {
        let err = FineTuningError::TrainingNotStarted;
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_total_batches_counter() {
        let config = make_config();
        let mut engine = FineTuningEngine::new(config);
        engine.start_epoch().unwrap();
        engine.record_batch(0.5, 1.0).unwrap();
        engine.record_batch(0.4, 0.9).unwrap();
        engine.start_epoch().unwrap();
        engine.record_batch(0.3, 0.8).unwrap();
        assert_eq!(engine.get_state().total_batches, 3);
    }
}
