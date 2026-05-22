//! Distributed Fine-Tuning — Coordinación de fine-tuning distribuido entre nodos
//!
//! Gestiona la coordinación de entrenamiento distribuido con sincronización
//! de gradientes, gestión de participantes y control de épocas.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::time::{Duration, Instant};

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum DistributedError {
    NodeAlreadyRegistered(String),
    NodeNotRegistered(String),
    TrainingNotStarted,
    TrainingAlreadyStarted,
    EpochNotStarted,
    InsufficientParticipants(usize),
    GradientSyncFailed(String),
    InvalidConfig(String),
}

impl fmt::Display for DistributedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DistributedError::NodeAlreadyRegistered(id) => {
                write!(f, "Node already registered: {}", id)
            }
            DistributedError::NodeNotRegistered(id) => {
                write!(f, "Node not registered: {}", id)
            }
            DistributedError::TrainingNotStarted => write!(f, "Training not started"),
            DistributedError::TrainingAlreadyStarted => write!(f, "Training already started"),
            DistributedError::EpochNotStarted => write!(f, "Epoch not started"),
            DistributedError::InsufficientParticipants(min) => {
                write!(f, "Insufficient participants (min: {})", min)
            }
            DistributedError::GradientSyncFailed(msg) => {
                write!(f, "Gradient sync failed: {}", msg)
            }
            DistributedError::InvalidConfig(msg) => {
                write!(f, "Invalid config: {}", msg)
            }
        }
    }
}

impl std::error::Error for DistributedError {}

// ============================================================================
// Training State
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum DistributedState {
    Idle,
    Starting,
    Training,
    Syncing,
    Completed,
    Failed(String),
}

impl fmt::Display for DistributedState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DistributedState::Idle => write!(f, "Idle"),
            DistributedState::Starting => write!(f, "Starting"),
            DistributedState::Training => write!(f, "Training"),
            DistributedState::Syncing => write!(f, "Syncing"),
            DistributedState::Completed => write!(f, "Completed"),
            DistributedState::Failed(msg) => write!(f, "Failed({})", msg),
        }
    }
}

// ============================================================================
// Participant Node
// ============================================================================

#[derive(Debug, Clone)]
pub struct ParticipantNode {
    pub node_id: String,
    pub compute_power: f32,
    pub gradient_dim: usize,
    pub last_heartbeat: Instant,
    pub gradients_submitted: usize,
    pub epochs_participated: usize,
    pub is_active: bool,
}

impl ParticipantNode {
    pub fn new(node_id: String, compute_power: f32, gradient_dim: usize) -> Self {
        Self {
            node_id,
            compute_power,
            gradient_dim,
            last_heartbeat: Instant::now(),
            gradients_submitted: 0,
            epochs_participated: 0,
            is_active: true,
        }
    }

    pub fn heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
    }

    pub fn is_stale(&self, max_stale: Duration) -> bool {
        self.last_heartbeat.elapsed() > max_stale
    }

    pub fn record_gradient_submission(&mut self) {
        self.gradients_submitted += 1;
    }

    pub fn record_epoch_participation(&mut self) {
        self.epochs_participated += 1;
    }
}

// ============================================================================
// Gradient Batch
// ============================================================================

#[derive(Debug, Clone)]
pub struct GradientBatch {
    pub node_id: String,
    pub epoch: usize,
    pub batch_idx: usize,
    pub gradients: Vec<f32>,
    pub loss: f32,
    pub timestamp: Instant,
}

impl GradientBatch {
    pub fn new(
        node_id: String,
        epoch: usize,
        batch_idx: usize,
        gradients: Vec<f32>,
        loss: f32,
    ) -> Self {
        Self {
            node_id,
            epoch,
            batch_idx,
            gradients,
            loss,
            timestamp: Instant::now(),
        }
    }

    pub fn norm(&self) -> f32 {
        self.gradients.iter().map(|g| g * g).sum::<f32>().sqrt()
    }
}

// ============================================================================
// Epoch Summary
// ============================================================================

#[derive(Debug, Clone)]
pub struct EpochSummary {
    pub epoch: usize,
    pub participants: HashSet<String>,
    pub avg_loss: f32,
    pub avg_gradient_norm: f32,
    pub aggregated_gradients: Vec<f32>,
    pub duration: Duration,
    pub started_at: Instant,
}

impl EpochSummary {
    pub fn new(epoch: usize, started_at: Instant) -> Self {
        Self {
            epoch,
            participants: HashSet::new(),
            avg_loss: 0.0,
            avg_gradient_norm: 0.0,
            aggregated_gradients: Vec::new(),
            duration: Duration::ZERO,
            started_at,
        }
    }
}

// ============================================================================
// Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct DistributedConfig {
    pub min_participants: usize,
    pub max_participants: usize,
    pub gradient_dim: usize,
    pub sync_timeout: Duration,
    pub heartbeat_timeout: Duration,
    pub aggregation_method: AggregationMethod,
}

impl DistributedConfig {
    pub fn new(min_participants: usize, gradient_dim: usize) -> Self {
        Self {
            min_participants,
            max_participants: 100,
            gradient_dim,
            sync_timeout: Duration::from_secs(30),
            heartbeat_timeout: Duration::from_secs(10),
            aggregation_method: AggregationMethod::FedAvg,
        }
    }

    pub fn validate(&self) -> Result<(), DistributedError> {
        if self.min_participants == 0 {
            return Err(DistributedError::InvalidConfig(
                "Min participants must be > 0".to_string(),
            ));
        }
        if self.gradient_dim == 0 {
            return Err(DistributedError::InvalidConfig(
                "Gradient dimension must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for DistributedConfig {
    fn default() -> Self {
        Self::new(3, 128)
    }
}

// ============================================================================
// Aggregation Method
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum AggregationMethod {
    FedAvg,
    FedAvgWeighted,
    Krum,
    MultiKrum { k: usize },
}

impl fmt::Display for AggregationMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AggregationMethod::FedAvg => write!(f, "FedAvg"),
            AggregationMethod::FedAvgWeighted => write!(f, "FedAvgWeighted"),
            AggregationMethod::Krum => write!(f, "Krum"),
            AggregationMethod::MultiKrum { k } => write!(f, "MultiKrum(k={})", k),
        }
    }
}

// ============================================================================
// Distributed Fine-Tuning Engine
// ============================================================================

pub struct DistributedFineTuning {
    config: DistributedConfig,
    state: DistributedState,
    participants: HashMap<String, ParticipantNode>,
    pending_gradients: Vec<GradientBatch>,
    epoch_summaries: Vec<EpochSummary>,
    current_epoch: usize,
    started_at: Option<Instant>,
}

impl DistributedFineTuning {
    pub fn new(config: DistributedConfig) -> Self {
        Self {
            config,
            state: DistributedState::Idle,
            participants: HashMap::new(),
            pending_gradients: Vec::new(),
            epoch_summaries: Vec::new(),
            current_epoch: 0,
            started_at: None,
        }
    }

    // ------------------------------------------------------------------
    // Participant Management
    // ------------------------------------------------------------------

    pub fn register_node(
        &mut self,
        node_id: String,
        compute_power: f32,
        gradient_dim: usize,
    ) -> Result<(), DistributedError> {
        if self.state != DistributedState::Idle {
            return Err(DistributedError::TrainingAlreadyStarted);
        }
        if self.participants.contains_key(&node_id) {
            return Err(DistributedError::NodeAlreadyRegistered(node_id));
        }
        self.participants.insert(
            node_id.clone(),
            ParticipantNode::new(node_id, compute_power, gradient_dim),
        );
        Ok(())
    }

    pub fn unregister_node(&mut self, node_id: &str) -> Result<(), DistributedError> {
        if !self.participants.contains_key(node_id) {
            return Err(DistributedError::NodeNotRegistered(node_id.to_string()));
        }
        self.participants.remove(node_id);
        Ok(())
    }

    pub fn heartbeat_node(&mut self, node_id: &str) -> Result<(), DistributedError> {
        let node = self
            .participants
            .get_mut(node_id)
            .ok_or(DistributedError::NodeNotRegistered(node_id.to_string()))?;
        node.heartbeat();
        Ok(())
    }

    pub fn detect_stale_nodes(&mut self) -> Vec<String> {
        let mut stale = Vec::new();
        for (id, node) in &mut self.participants {
            if node.is_stale(self.config.heartbeat_timeout) {
                node.is_active = false;
                stale.push(id.clone());
            }
        }
        stale
    }

    pub fn active_participant_count(&self) -> usize {
        self.participants.values().filter(|n| n.is_active).count()
    }

    // ------------------------------------------------------------------
    // Training Lifecycle
    // ------------------------------------------------------------------

    pub fn start_training(&mut self) -> Result<(), DistributedError> {
        if self.state != DistributedState::Idle {
            return Err(DistributedError::TrainingAlreadyStarted);
        }
        let active = self.active_participant_count();
        if active < self.config.min_participants {
            return Err(DistributedError::InsufficientParticipants(
                self.config.min_participants,
            ));
        }
        self.state = DistributedState::Starting;
        self.started_at = Some(Instant::now());
        self.state = DistributedState::Training;
        Ok(())
    }

    pub fn start_epoch(&mut self) -> Result<(), DistributedError> {
        if self.state != DistributedState::Training {
            return Err(DistributedError::TrainingNotStarted);
        }
        self.current_epoch += 1;
        self.pending_gradients.clear();
        self.state = DistributedState::Training;
        Ok(())
    }

    pub fn submit_gradient(&mut self, batch: GradientBatch) -> Result<(), DistributedError> {
        if self.state != DistributedState::Training {
            return Err(DistributedError::TrainingNotStarted);
        }
        if batch.epoch != self.current_epoch {
            return Err(DistributedError::EpochNotStarted);
        }
        // Record submission
        if let Some(node) = self.participants.get_mut(&batch.node_id) {
            node.record_gradient_submission();
        }
        self.pending_gradients.push(batch);
        Ok(())
    }

    pub fn sync_gradients(&mut self) -> Result<EpochSummary, DistributedError> {
        if self.state != DistributedState::Training {
            return Err(DistributedError::TrainingNotStarted);
        }
        self.state = DistributedState::Syncing;

        let started_at = self.started_at.unwrap_or_else(Instant::now);

        let mut summary = EpochSummary::new(self.current_epoch, started_at);

        // Collect participants for this epoch
        let mut losses = Vec::new();
        let mut norms = Vec::new();

        for batch in &self.pending_gradients {
            summary.participants.insert(batch.node_id.clone());
            losses.push(batch.loss);
            norms.push(batch.norm());
        }

        // Compute averages
        if !losses.is_empty() {
            summary.avg_loss = losses.iter().sum::<f32>() / losses.len() as f32;
        }
        if !norms.is_empty() {
            summary.avg_gradient_norm = norms.iter().sum::<f32>() / norms.len() as f32;
        }

        // Aggregate gradients
        summary.aggregated_gradients = self.aggregate_gradients();

        // Update duration
        summary.duration = started_at.elapsed();

        // Record epoch participation
        for pid in &summary.participants {
            if let Some(node) = self.participants.get_mut(pid) {
                node.record_epoch_participation();
            }
        }

        self.epoch_summaries.push(summary.clone());
        self.state = DistributedState::Training;

        Ok(summary)
    }

    pub fn complete_training(&mut self) {
        self.state = DistributedState::Completed;
    }

    pub fn fail_training(&mut self, reason: &str) {
        self.state = DistributedState::Failed(reason.to_string());
    }

    // ------------------------------------------------------------------
    // Gradient Aggregation
    // ------------------------------------------------------------------

    fn aggregate_gradients(&self) -> Vec<f32> {
        match self.config.aggregation_method {
            AggregationMethod::FedAvg | AggregationMethod::FedAvgWeighted => {
                self.fed_avg_aggregate()
            }
            AggregationMethod::Krum => self.krum_aggregate(),
            AggregationMethod::MultiKrum { .. } => self.krum_aggregate(), // Simplified
        }
    }

    fn fed_avg_aggregate(&self) -> Vec<f32> {
        if self.pending_gradients.is_empty() {
            return vec![0.0; self.config.gradient_dim];
        }

        let n = self.pending_gradients.len();
        let mut result = vec![0.0; self.config.gradient_dim];

        for batch in &self.pending_gradients {
            for (i, g) in batch.gradients.iter().enumerate() {
                if i < result.len() {
                    result[i] += g / n as f32;
                }
            }
        }

        result
    }

    fn krum_aggregate(&self) -> Vec<f32> {
        if self.pending_gradients.len() <= 1 {
            return self.fed_avg_aggregate();
        }

        // Simplified Krum: select gradient closest to others on average
        let n = self.pending_gradients.len();
        let mut min_distance_idx = 0;
        let mut min_distance = f32::MAX;

        for i in 0..n {
            let mut total_distance = 0.0;
            for j in 0..n {
                if i != j {
                    total_distance += self.gradient_distance(i, j);
                }
            }
            if total_distance < min_distance {
                min_distance = total_distance;
                min_distance_idx = i;
            }
        }

        self.pending_gradients[min_distance_idx].gradients.clone()
    }

    fn gradient_distance(&self, idx_a: usize, idx_b: usize) -> f32 {
        let a = &self.pending_gradients[idx_a].gradients;
        let b = &self.pending_gradients[idx_b].gradients;
        let len = a.len().min(b.len());

        (0..len).map(|i| (a[i] - b[i]).abs()).sum::<f32>()
    }

    // ------------------------------------------------------------------
    // Queries
    // ------------------------------------------------------------------

    pub fn get_state(&self) -> &DistributedState {
        &self.state
    }

    pub fn get_current_epoch(&self) -> usize {
        self.current_epoch
    }

    pub fn get_participant(&self, node_id: &str) -> Option<&ParticipantNode> {
        self.participants.get(node_id)
    }

    pub fn get_epoch_summaries(&self) -> &[EpochSummary] {
        &self.epoch_summaries
    }

    pub fn get_pending_gradient_count(&self) -> usize {
        self.pending_gradients.len()
    }

    pub fn get_total_duration(&self) -> Duration {
        self.started_at
            .map(|s| s.elapsed())
            .unwrap_or(Duration::ZERO)
    }
}

impl Default for DistributedFineTuning {
    fn default() -> Self {
        Self::new(DistributedConfig::default())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gradient(dim: usize, value: f32) -> Vec<f32> {
        vec![value; dim]
    }

    fn make_batch(node_id: &str, epoch: usize, dim: usize, loss: f32) -> GradientBatch {
        GradientBatch::new(node_id.to_string(), epoch, 0, make_gradient(dim, 0.1), loss)
    }

    #[test]
    fn test_engine_creation() {
        let config = DistributedConfig::new(2, 64);
        let engine = DistributedFineTuning::new(config);
        assert_eq!(*engine.get_state(), DistributedState::Idle);
        assert_eq!(engine.get_current_epoch(), 0);
    }

    #[test]
    fn test_register_node() {
        let mut engine = DistributedFineTuning::default();
        assert!(engine.register_node("node-1".to_string(), 1.0, 128).is_ok());
        assert!(engine.get_participant("node-1").is_some());
    }

    #[test]
    fn test_register_duplicate_node() {
        let mut engine = DistributedFineTuning::default();
        engine
            .register_node("node-1".to_string(), 1.0, 128)
            .unwrap();
        match engine.register_node("node-1".to_string(), 1.0, 128) {
            Err(DistributedError::NodeAlreadyRegistered(id)) => {
                assert_eq!(id, "node-1");
            }
            _ => panic!("Expected NodeAlreadyRegistered"),
        }
    }

    #[test]
    fn test_unregister_node() {
        let mut engine = DistributedFineTuning::default();
        engine
            .register_node("node-1".to_string(), 1.0, 128)
            .unwrap();
        assert!(engine.unregister_node("node-1").is_ok());
        assert!(engine.get_participant("node-1").is_none());
    }

    #[test]
    fn test_start_training_insufficient_participants() {
        let config = DistributedConfig::new(3, 64);
        let mut engine = DistributedFineTuning::new(config);
        engine.register_node("node-1".to_string(), 1.0, 64).unwrap();
        engine.register_node("node-2".to_string(), 1.0, 64).unwrap();

        match engine.start_training() {
            Err(DistributedError::InsufficientParticipants(min)) => {
                assert_eq!(min, 3);
            }
            _ => panic!("Expected InsufficientParticipants"),
        }
    }

    #[test]
    fn test_start_training_success() {
        let config = DistributedConfig::new(2, 64);
        let mut engine = DistributedFineTuning::new(config);
        engine.register_node("node-1".to_string(), 1.0, 64).unwrap();
        engine.register_node("node-2".to_string(), 1.0, 64).unwrap();

        assert!(engine.start_training().is_ok());
        assert_eq!(*engine.get_state(), DistributedState::Training);
    }

    #[test]
    fn test_submit_and_sync_gradients() {
        let config = DistributedConfig::new(2, 64);
        let mut engine = DistributedFineTuning::new(config);
        engine.register_node("node-1".to_string(), 1.0, 64).unwrap();
        engine.register_node("node-2".to_string(), 1.0, 64).unwrap();
        engine.start_training().unwrap();
        engine.start_epoch().unwrap();

        engine
            .submit_gradient(make_batch("node-1", 1, 64, 0.5))
            .unwrap();
        engine
            .submit_gradient(make_batch("node-2", 1, 64, 0.3))
            .unwrap();

        let summary = engine.sync_gradients().unwrap();
        assert_eq!(summary.epoch, 1);
        assert_eq!(summary.participants.len(), 2);
        assert!((summary.avg_loss - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_fed_avg_aggregation() {
        let config = DistributedConfig::new(1, 4);
        let mut engine = DistributedFineTuning::new(config);
        engine.register_node("node-1".to_string(), 1.0, 4).unwrap();
        engine.start_training().unwrap();
        engine.start_epoch().unwrap();

        // Submit gradients [1, 2, 3, 4] and [3, 4, 5, 6]
        let batch1 = GradientBatch::new("node-1".into(), 1, 0, vec![1.0, 2.0, 3.0, 4.0], 0.5);
        let batch2 = GradientBatch::new("node-1".into(), 1, 1, vec![3.0, 4.0, 5.0, 6.0], 0.3);
        engine.submit_gradient(batch1).unwrap();
        engine.submit_gradient(batch2).unwrap();

        let summary = engine.sync_gradients().unwrap();
        // FedAvg: ([1,2,3,4] + [3,4,5,6]) / 2 = [2, 3, 4, 5]
        assert_eq!(summary.aggregated_gradients, vec![2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_heartbeat() {
        let mut engine = DistributedFineTuning::default();
        engine
            .register_node("node-1".to_string(), 1.0, 128)
            .unwrap();
        assert!(engine.heartbeat_node("node-1").is_ok());
    }

    #[test]
    fn test_complete_training() {
        let config = DistributedConfig::new(1, 32);
        let mut engine = DistributedFineTuning::new(config);
        engine.register_node("node-1".to_string(), 1.0, 32).unwrap();
        engine.start_training().unwrap();
        engine.complete_training();
        assert_eq!(*engine.get_state(), DistributedState::Completed);
    }

    #[test]
    fn test_fail_training() {
        let mut engine = DistributedFineTuning::default();
        engine.fail_training("test failure");
        assert_eq!(
            *engine.get_state(),
            DistributedState::Failed("test failure".to_string())
        );
    }

    #[test]
    fn test_gradient_batch_norm() {
        let batch = make_batch("node-1", 1, 3, 0.5);
        // norm of [0.1, 0.1, 0.1] = sqrt(0.03) ~ 0.1732
        let expected = (0.1_f32 * 0.1 * 3.0).sqrt();
        assert!((batch.norm() - expected).abs() < 0.001);
    }

    #[test]
    fn test_config_validation() {
        let mut config = DistributedConfig::new(1, 64);
        assert!(config.validate().is_ok());

        config.min_participants = 0;
        match config.validate() {
            Err(DistributedError::InvalidConfig(msg)) => {
                assert!(msg.contains("Min participants"));
            }
            _ => panic!("Expected InvalidConfig"),
        }

        config.min_participants = 1;
        config.gradient_dim = 0;
        match config.validate() {
            Err(DistributedError::InvalidConfig(msg)) => {
                assert!(msg.contains("Gradient dimension"));
            }
            _ => panic!("Expected InvalidConfig"),
        }
    }

    #[test]
    fn test_active_participant_count() {
        let mut engine = DistributedFineTuning::default();
        engine.register_node("node-1".to_string(), 1.0, 64).unwrap();
        engine.register_node("node-2".to_string(), 1.0, 64).unwrap();
        assert_eq!(engine.active_participant_count(), 2);
    }

    #[test]
    fn test_state_display() {
        assert_eq!(format!("{}", DistributedState::Idle), "Idle");
        assert_eq!(format!("{}", DistributedState::Training), "Training");
        assert_eq!(
            format!("{}", DistributedState::Failed("err".into())),
            "Failed(err)"
        );
    }

    #[test]
    fn test_aggregation_method_display() {
        assert_eq!(format!("{}", AggregationMethod::FedAvg), "FedAvg");
        assert_eq!(
            format!("{}", AggregationMethod::MultiKrum { k: 3 }),
            "MultiKrum(k=3)"
        );
    }

    #[test]
    fn test_error_display() {
        match DistributedError::NodeNotRegistered("x".into()) {
            e => assert!(format!("{}", e).contains("x")),
            _ => {}
        }
    }

    #[test]
    fn test_epoch_summary_collection() {
        let config = DistributedConfig::new(1, 32);
        let mut engine = DistributedFineTuning::new(config);
        engine.register_node("node-1".to_string(), 1.0, 32).unwrap();
        engine.start_training().unwrap();

        for _epoch in 1..=3 {
            engine.start_epoch().unwrap();
            engine
                .submit_gradient(make_batch("node-1", _epoch, 32, 0.5))
                .unwrap();
            engine.sync_gradients().unwrap();
        }

        assert_eq!(engine.get_epoch_summaries().len(), 3);
    }

    #[test]
    fn test_krum_aggregation() {
        let mut config = DistributedConfig::new(1, 4);
        config.aggregation_method = AggregationMethod::Krum;
        let mut engine = DistributedFineTuning::new(config);
        engine.register_node("n1".to_string(), 1.0, 4).unwrap();
        engine.register_node("n2".to_string(), 1.0, 4).unwrap();
        engine.register_node("n3".to_string(), 1.0, 4).unwrap();
        engine.start_training().unwrap();
        engine.start_epoch().unwrap();

        // n1 and n2 agree, n3 is outlier
        engine
            .submit_gradient(GradientBatch::new(
                "n1".into(),
                1,
                0,
                vec![1.0, 1.0, 1.0, 1.0],
                0.5,
            ))
            .unwrap();
        engine
            .submit_gradient(GradientBatch::new(
                "n2".into(),
                1,
                0,
                vec![1.1, 1.1, 1.1, 1.1],
                0.5,
            ))
            .unwrap();
        engine
            .submit_gradient(GradientBatch::new(
                "n3".into(),
                1,
                0,
                vec![10.0, 10.0, 10.0, 10.0],
                0.9,
            ))
            .unwrap();

        let summary = engine.sync_gradients().unwrap();
        // Krum should pick n1 or n2 (closest to others)
        assert!(summary.aggregated_gradients[0] < 5.0);
    }

    #[test]
    fn test_config_default() {
        let config = DistributedConfig::default();
        assert_eq!(config.min_participants, 3);
        assert_eq!(config.gradient_dim, 128);
    }

    #[test]
    fn test_engine_default() {
        let engine = DistributedFineTuning::default();
        assert_eq!(*engine.get_state(), DistributedState::Idle);
    }

    #[test]
    fn test_total_duration() {
        let mut engine = DistributedFineTuning::default();
        // Register 3 nodes to meet min_participants requirement (default=3)
        engine.register_node("n1".to_string(), 1.0, 32).unwrap();
        engine.register_node("n2".to_string(), 1.0, 32).unwrap();
        engine.register_node("n3".to_string(), 1.0, 32).unwrap();
        engine.start_training().unwrap();
        let duration = engine.get_total_duration();
        assert!(duration.as_millis() >= 0);
    }

    #[test]
    fn test_participant_node_stale() {
        let mut node = ParticipantNode::new("test".to_string(), 1.0, 64);
        assert!(!node.is_stale(Duration::from_secs(60)));

        // Simulate stale by setting old heartbeat
        node.last_heartbeat = Instant::now() - Duration::from_secs(120);
        assert!(node.is_stale(Duration::from_secs(60)));
    }

    #[test]
    fn test_detect_stale_nodes() {
        let mut engine = DistributedFineTuning::default();
        engine.config.heartbeat_timeout = Duration::from_millis(100);
        engine.register_node("n1".to_string(), 1.0, 32).unwrap();

        std::thread::sleep(Duration::from_millis(150));
        let stale = engine.detect_stale_nodes();
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0], "n1");
    }

    #[test]
    fn test_submit_gradient_wrong_epoch() {
        let config = DistributedConfig::new(1, 32);
        let mut engine = DistributedFineTuning::new(config);
        engine.register_node("n1".to_string(), 1.0, 32).unwrap();
        engine.start_training().unwrap();
        engine.start_epoch().unwrap(); // epoch = 1

        let batch = make_batch("n1", 2, 32, 0.5); // wrong epoch
        match engine.submit_gradient(batch) {
            Err(DistributedError::EpochNotStarted) => {}
            _ => panic!("Expected EpochNotStarted"),
        }
    }

    #[test]
    fn test_gradient_dimension_mismatch() {
        let config = DistributedConfig::new(1, 64);
        let mut engine = DistributedFineTuning::new(config);
        engine.register_node("n1".to_string(), 1.0, 64).unwrap();
        engine.start_training().unwrap();
        engine.start_epoch().unwrap();

        // Submit gradient with different dimension (should still work, aggregation handles it)
        let batch = GradientBatch::new("n1".into(), 1, 0, vec![0.1; 32], 0.5);
        assert!(engine.submit_gradient(batch).is_ok());
    }
}
