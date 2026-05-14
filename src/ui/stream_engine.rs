//! Stream Engine — Real-time metric streaming with backpressure and subscriber management.
//!
//! LP-106: UI Dashboard v5 & Real-time Streams
//! Provides real-time metric streaming capabilities for Dashboard v5 with support for
//! multiple subscribers, backpressure control, windowed aggregation, and stream filtering.
//!
//! Características:
//! - Suscriptores con buffers acotados para backpressure
//! - Filtrado por categoría de métrica (ZKP v7, Cross-Pool, Governance v4, System)
//! - Agregación en ventanas temporales (min/max/avg/count)
//! - Priorización de streams críticos
//! - Detección de suscriptores lentos con auto-pause
//!
//! Protegido con `#[cfg(feature = "v1.4-sprint2")]`.

use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::cmp::Ordering;
use thiserror::Error;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum StreamError {
    #[error("Suscriptor no encontrado: {0}")]
    SubscriberNotFound(String),
    #[error("Stream ya existe: {0}")]
    StreamAlreadyExists(String),
    #[error("Buffer lleno para suscriptor: {0}")]
    BufferFull(String),
    #[error("Categoría inválida: {0}")]
    InvalidCategory(String),
    #[error("Límite de suscriptores alcanzado")]
    MaxSubscribersReached,
}

// ─── Metric Category ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MetricCategory {
    ZkpV7,
    CrossPool,
    GovernanceV4,
    Network,
    System,
    Custom(String),
}

impl std::fmt::Display for MetricCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricCategory::ZkpV7 => write!(f, "zkp_v7"),
            MetricCategory::CrossPool => write!(f, "cross_pool"),
            MetricCategory::GovernanceV4 => write!(f, "governance_v4"),
            MetricCategory::Network => write!(f, "network"),
            MetricCategory::System => write!(f, "system"),
            MetricCategory::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

// ─── Stream Event ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    /// Event sequence number.
    pub sequence: u64,
    /// Metric category.
    pub category: MetricCategory,
    /// Metric name.
    pub metric_name: String,
    /// Metric value.
    pub value: f64,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Priority level (0-255, higher = more urgent).
    pub priority: u8,
}

impl StreamEvent {
    pub fn new(
        sequence: u64,
        category: MetricCategory,
        metric_name: String,
        value: f64,
        priority: u8,
    ) -> Self {
        Self {
            sequence,
            category,
            metric_name,
            value,
            timestamp_ms: current_timestamp_ms(),
            priority,
        }
    }
}

// ─── Priority Event for BinaryHeap ───────────────────────────────────────────

#[derive(Debug, Clone)]
struct PriorityEvent {
    event: StreamEvent,
}

impl PartialEq for PriorityEvent {
    fn eq(&self, other: &Self) -> bool {
        self.event.priority == other.event.priority
            && self.event.timestamp_ms == other.event.timestamp_ms
    }
}

impl Eq for PriorityEvent {}

impl Ord for PriorityEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        self.event.priority.cmp(&other.event.priority)
            .then_with(|| self.event.timestamp_ms.cmp(&other.event.timestamp_ms))
    }
}

impl PartialOrd for PriorityEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ─── Subscriber ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriberConfig {
    /// Subscriber identifier.
    pub id: String,
    /// Buffer size for backpressure.
    pub buffer_size: usize,
    /// Subscribed categories.
    pub categories: Vec<MetricCategory>,
    /// Auto-pause when buffer reaches this percentage (0.0-1.0).
    pub pause_threshold: f64,
    /// Resume when buffer drops below this percentage (0.0-1.0).
    pub resume_threshold: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SubscriberState {
    Active,
    Paused,
    Dropped,
}

impl std::fmt::Display for SubscriberState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriberState::Active => write!(f, "active"),
            SubscriberState::Paused => write!(f, "paused"),
            SubscriberState::Dropped => write!(f, "dropped"),
        }
    }
}

/// Managed subscriber with buffer and state.
pub struct Subscriber {
    /// Subscriber configuration.
    config: SubscriberConfig,
    /// Current state.
    state: SubscriberState,
    /// Event buffer.
    buffer: VecDeque<StreamEvent>,
    /// Total events delivered.
    events_delivered: u64,
    /// Total events dropped.
    events_dropped: u64,
}

impl Subscriber {
    pub fn new(config: SubscriberConfig) -> Self {
        Self {
            config,
            state: SubscriberState::Active,
            buffer: VecDeque::with_capacity(100),
            events_delivered: 0,
            events_dropped: 0,
        }
    }

    pub fn id(&self) -> &str {
        &self.config.id
    }

    pub fn state(&self) -> &SubscriberState {
        &self.state
    }

    pub fn buffer_usage(&self) -> f64 {
        if self.config.buffer_size == 0 {
            return 1.0;
        }
        self.buffer.len() as f64 / self.config.buffer_size as f64
    }

    pub fn enqueue(&mut self, event: StreamEvent) -> Result<(), StreamError> {
        // Check if subscriber should be auto-paused
        if self.state == SubscriberState::Active && self.buffer_usage() >= self.config.pause_threshold {
            self.state = SubscriberState::Paused;
        }
        if self.state != SubscriberState::Active {
            return Err(StreamError::BufferFull(self.config.id.clone()));
        }
        if self.buffer.len() >= self.config.buffer_size {
            return Err(StreamError::BufferFull(self.config.id.clone()));
        }
        self.buffer.push_back(event);
        Ok(())
    }

    pub fn dequeue(&mut self) -> Option<StreamEvent> {
        let event = self.buffer.pop_front();
        if event.is_some() {
            self.events_delivered += 1;
            // Check if subscriber should resume
            if self.state == SubscriberState::Paused && self.buffer_usage() < self.config.resume_threshold {
                self.state = SubscriberState::Active;
            }
        }
        event
    }

    pub fn drain(&mut self, max: usize) -> Vec<StreamEvent> {
        let count = max.min(self.buffer.len());
        let mut events = Vec::with_capacity(count);
        for _ in 0..count {
            if let Some(event) = self.dequeue() {
                events.push(event);
            }
        }
        events
    }

    pub fn matches_category(&self, category: &MetricCategory) -> bool {
        self.config.categories.is_empty() || self.config.categories.contains(category)
    }
}

impl std::fmt::Debug for Subscriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscriber")
            .field("id", &self.config.id)
            .field("state", &self.state)
            .field("buffer_len", &self.buffer.len())
            .field("events_delivered", &self.events_delivered)
            .field("events_dropped", &self.events_dropped)
            .finish()
    }
}

// ─── Window Aggregation ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowStats {
    /// Minimum value in window.
    pub min: f64,
    /// Maximum value in window.
    pub max: f64,
    /// Average value in window.
    pub avg: f64,
    /// Count of values in window.
    pub count: u64,
    /// Window start timestamp.
    pub window_start_ms: u64,
    /// Window end timestamp.
    pub window_end_ms: u64,
}

impl WindowStats {
    pub fn new() -> Self {
        Self {
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            avg: 0.0,
            count: 0,
            window_start_ms: 0,
            window_end_ms: 0,
        }
    }

    pub fn record(&mut self, value: f64, timestamp_ms: u64) {
        if self.count == 0 {
            self.window_start_ms = timestamp_ms;
        }
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        let old_sum = self.avg * self.count as f64;
        self.count += 1;
        self.avg = (old_sum + value) / self.count as f64;
        self.window_end_ms = timestamp_ms;
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl Default for WindowStats {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Stream Config ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Maximum number of subscribers.
    pub max_subscribers: usize,
    /// Default buffer size for subscribers.
    pub default_buffer_size: usize,
    /// Event history size.
    pub event_history_size: usize,
    /// Window size in milliseconds for aggregation.
    pub window_size_ms: u64,
    /// Enable priority ordering.
    pub priority_ordering: bool,
    /// Enable auto-pause for slow subscribers.
    pub auto_pause: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            max_subscribers: 64,
            default_buffer_size: 256,
            event_history_size: 1000,
            window_size_ms: 5000,
            priority_ordering: true,
            auto_pause: true,
        }
    }
}

// ─── Stream Stats ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStats {
    /// Total events published.
    pub events_published: u64,
    /// Total events delivered.
    pub events_delivered: u64,
    /// Total events dropped.
    pub events_dropped: u64,
    /// Active subscriber count.
    pub active_subscribers: usize,
    /// Paused subscriber count.
    pub paused_subscribers: usize,
    /// Current priority queue depth.
    pub queue_depth: usize,
}

impl Default for StreamStats {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamStats {
    pub fn new() -> Self {
        Self {
            events_published: 0,
            events_delivered: 0,
            events_dropped: 0,
            active_subscribers: 0,
            paused_subscribers: 0,
            queue_depth: 0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

// ─── Stream Engine ───────────────────────────────────────────────────────────

/// Real-time metric streaming engine.
pub struct StreamEngine {
    /// Configuration.
    config: StreamConfig,
    /// Registered subscribers.
    subscribers: HashMap<String, Subscriber>,
    /// Priority queue for events.
    priority_queue: BinaryHeap<PriorityEvent>,
    /// Event history.
    history: VecDeque<StreamEvent>,
    /// Window aggregation per category.
    windows: HashMap<String, WindowStats>,
    /// Sequence counter.
    sequence: u64,
    /// Statistics.
    stats: StreamStats,
}

impl StreamEngine {
    /// Create a new stream engine with default config.
    pub fn new() -> Self {
        Self::with_config(StreamConfig::default())
    }

    /// Create a new stream engine with custom config.
    pub fn with_config(config: StreamConfig) -> Self {
        Self {
            config,
            subscribers: HashMap::new(),
            priority_queue: BinaryHeap::new(),
            history: VecDeque::with_capacity(1000),
            windows: HashMap::new(),
            sequence: 0,
            stats: StreamStats::new(),
        }
    }

    /// Register a new subscriber.
    pub fn subscribe(&mut self, config: SubscriberConfig) -> Result<(), StreamError> {
        if self.subscribers.len() >= self.config.max_subscribers {
            return Err(StreamError::MaxSubscribersReached);
        }
        if self.subscribers.contains_key(&config.id) {
            return Err(StreamError::StreamAlreadyExists(config.id));
        }
        self.subscribers.insert(config.id.clone(), Subscriber::new(config));
        Ok(())
    }

    /// Unregister a subscriber.
    pub fn unsubscribe(&mut self, id: &str) -> Result<(), StreamError> {
        self.subscribers
            .remove(id)
            .ok_or_else(|| StreamError::SubscriberNotFound(id.to_string()))?;
        Ok(())
    }

    /// Get subscriber state.
    pub fn get_subscriber_state(&self, id: &str) -> Option<&SubscriberState> {
        self.subscribers.get(id).map(|s| s.state())
    }

    /// Publish a metric event.
    pub fn publish(
        &mut self,
        category: MetricCategory,
        metric_name: String,
        value: f64,
        priority: u8,
    ) {
        self.sequence += 1;
        let event = StreamEvent::new(self.sequence, category.clone(), metric_name, value, priority);
        self.stats.events_published += 1;

        // Add to priority queue or directly
        if self.config.priority_ordering {
            self.priority_queue.push(PriorityEvent { event: event.clone() });
        }

        // Update window stats
        let window_key = format!("{}", category);
        let window = self.windows.entry(window_key).or_insert_with(WindowStats::new);
        window.record(value, event.timestamp_ms);

        // Dispatch to subscribers
        self.dispatch_event(event);
    }

    /// Process pending events from priority queue.
    pub fn process_queue(&mut self) -> usize {
        let mut processed = 0;
        while let Some(priority_event) = self.priority_queue.pop() {
            let event = priority_event.event;
            self.dispatch_event(event);
            processed += 1;
        }
        processed
    }

    /// Get events for a subscriber.
    pub fn get_events(&mut self, subscriber_id: &str, max: usize) -> Vec<StreamEvent> {
        if let Some(subscriber) = self.subscribers.get_mut(subscriber_id) {
            subscriber.drain(max)
        } else {
            Vec::new()
        }
    }

    /// Get window stats for a category.
    pub fn get_window_stats(&self, category: &MetricCategory) -> Option<&WindowStats> {
        self.windows.get(&format!("{}", category))
    }

    /// Get recent event history.
    pub fn get_history(&self, count: usize) -> Vec<&StreamEvent> {
        let available = count.min(self.history.len());
        let start = self.history.len() - available;
        self.history[start..].iter().collect()
    }

    /// Get active subscriber count.
    pub fn active_subscriber_count(&self) -> usize {
        self.subscribers.values().filter(|s| *s.state() == SubscriberState::Active).count()
    }

    /// Get paused subscriber count.
    pub fn paused_subscriber_count(&self) -> usize {
        self.subscribers.values().filter(|s| *s.state() == SubscriberState::Paused).count()
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats.reset();
    }

    /// Get stats reference.
    pub fn get_stats(&self) -> &StreamStats {
        &self.stats
    }

    /// Get config reference.
    pub fn get_config(&self) -> &StreamConfig {
        &self.config
    }

    /// Force resume a paused subscriber.
    pub fn resume_subscriber(&mut self, id: &str) -> Result<(), StreamError> {
        let subscriber = self.subscribers
            .get_mut(id)
            .ok_or_else(|| StreamError::SubscriberNotFound(id.to_string()))?;
        subscriber.state = SubscriberState::Active;
        Ok(())
    }

    /// Clear event history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Clear all windows.
    pub fn clear_windows(&mut self) {
        self.windows.clear();
    }

    // ─── Private helpers ─────────────────────────────────────────────────────

    fn dispatch_event(&mut self, event: StreamEvent) {
        // Record in history
        self.history.push_back(event.clone());
        if self.history.len() > self.config.event_history_size {
            self.history.pop_front();
        }

        // Dispatch to matching subscribers
        for (_, subscriber) in self.subscribers.iter_mut() {
            if subscriber.matches_category(&event.category) {
                match subscriber.enqueue(event.clone()) {
                    Ok(()) => {
                        self.stats.events_delivered += 1;
                    }
                    Err(_) => {
                        self.stats.events_dropped += 1;
                        if let Some(sub) = self.subscribers.get_mut(subscriber.id()) {
                            sub.events_dropped += 1;
                        }
                    }
                }
            }
        }

        // Update stats
        self.stats.queue_depth = self.priority_queue.len();
        self.stats.active_subscribers = self.active_subscriber_count();
        self.stats.paused_subscribers = self.paused_subscriber_count();
    }
}

impl Default for StreamEngine {
    fn default() -> Self {
        Self::new()
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

    fn make_subscriber(id: &str, buffer_size: usize, categories: Vec<MetricCategory>) -> SubscriberConfig {
        SubscriberConfig {
            id: id.to_string(),
            buffer_size,
            categories,
            pause_threshold: 0.8,
            resume_threshold: 0.3,
        }
    }

    #[test]
    fn test_engine_creation() {
        let engine = StreamEngine::new();
        assert_eq!(engine.subscribers.len(), 0);
        assert_eq!(engine.stats.events_published, 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = StreamConfig {
            max_subscribers: 10,
            ..Default::default()
        };
        let engine = StreamEngine::with_config(config);
        assert_eq!(engine.config.max_subscribers, 10);
    }

    #[test]
    fn test_subscribe() {
        let mut engine = StreamEngine::new();
        engine.subscribe(make_subscriber("s1", 100, vec![])).unwrap();
        assert_eq!(engine.subscribers.len(), 1);
    }

    #[test]
    fn test_subscribe_duplicate() {
        let mut engine = StreamEngine::new();
        let config = make_subscriber("s1", 100, vec![]);
        engine.subscribe(config.clone()).unwrap();
        assert!(engine.subscribe(config).is_err());
    }

    #[test]
    fn test_max_subscribers() {
        let config = StreamConfig {
            max_subscribers: 2,
            ..Default::default()
        };
        let mut engine = StreamEngine::with_config(config);
        engine.subscribe(make_subscriber("s1", 10, vec![])).unwrap();
        engine.subscribe(make_subscriber("s2", 10, vec![])).unwrap();
        assert!(engine.subscribe(make_subscriber("s3", 10, vec![])).is_err());
    }

    #[test]
    fn test_unsubscribe() {
        let mut engine = StreamEngine::new();
        engine.subscribe(make_subscriber("s1", 100, vec![])).unwrap();
        engine.unsubscribe("s1").unwrap();
        assert_eq!(engine.subscribers.len(), 0);
    }

    #[test]
    fn test_unsubscribe_missing() {
        let mut engine = StreamEngine::new();
        assert!(engine.unsubscribe("nonexistent").is_err());
    }

    #[test]
    fn test_publish() {
        let mut engine = StreamEngine::new();
        engine.subscribe(make_subscriber("s1", 100, vec![])).unwrap();
        engine.publish(MetricCategory::ZkpV7, "proofs".to_string(), 42.0, 10);
        assert_eq!(engine.stats.events_published, 1);
    }

    #[test]
    fn test_publish_to_subscriber() {
        let mut engine = StreamEngine::new();
        engine.subscribe(make_subscriber("s1", 100, vec![MetricCategory::ZkpV7])).unwrap();
        engine.publish(MetricCategory::ZkpV7, "proofs".to_string(), 42.0, 10);
        let events = engine.get_events("s1", 10);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].value, 42.0);
    }

    #[test]
    fn test_category_filtering() {
        let mut engine = StreamEngine::new();
        engine.subscribe(make_subscriber("zkp", 100, vec![MetricCategory::ZkpV7])).unwrap();
        engine.publish(MetricCategory::ZkpV7, "proofs".to_string(), 10.0, 5);
        engine.publish(MetricCategory::System, "cpu".to_string(), 50.0, 5);
        let events = engine.get_events("zkp", 10);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].category, MetricCategory::ZkpV7);
    }

    #[test]
    fn test_wildcard_subscription() {
        let mut engine = StreamEngine::new();
        engine.subscribe(make_subscriber("all", 100, vec![])).unwrap();
        engine.publish(MetricCategory::ZkpV7, "proofs".to_string(), 10.0, 5);
        engine.publish(MetricCategory::System, "cpu".to_string(), 50.0, 5);
        let events = engine.get_events("all", 10);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_buffer_full() {
        let mut engine = StreamEngine::new();
        engine.subscribe(make_subscriber("s1", 2, vec![])).unwrap();
        engine.publish(MetricCategory::ZkpV7, "m".to_string(), 1.0, 5);
        engine.publish(MetricCategory::ZkpV7, "m".to_string(), 2.0, 5);
        engine.publish(MetricCategory::ZkpV7, "m".to_string(), 3.0, 5);
        assert_eq!(engine.stats.events_dropped, 1);
    }

    #[test]
    fn test_auto_pause() {
        let mut engine = StreamEngine::new();
        let config = SubscriberConfig {
            id: "s1".to_string(),
            buffer_size: 5,
            categories: vec![],
            pause_threshold: 0.4,
            resume_threshold: 0.2,
        };
        engine.subscribe(config).unwrap();
        for i in 0..3 {
            engine.publish(MetricCategory::ZkpV7, "m".to_string(), i as f64, 5);
        }
        assert_eq!(*engine.get_subscriber_state("s1").unwrap(), SubscriberState::Paused);
    }

    #[test]
    fn test_resume_subscriber() {
        let mut engine = StreamEngine::new();
        let config = SubscriberConfig {
            id: "s1".to_string(),
            buffer_size: 5,
            categories: vec![],
            pause_threshold: 0.4,
            resume_threshold: 0.2,
        };
        engine.subscribe(config).unwrap();
        for i in 0..3 {
            engine.publish(MetricCategory::ZkpV7, "m".to_string(), i as f64, 5);
        }
        engine.resume_subscriber("s1").unwrap();
        assert_eq!(*engine.get_subscriber_state("s1").unwrap(), SubscriberState::Active);
    }

    #[test]
    fn test_get_events() {
        let mut engine = StreamEngine::new();
        engine.subscribe(make_subscriber("s1", 100, vec![])).unwrap();
        engine.publish(MetricCategory::ZkpV7, "m".to_string(), 1.0, 5);
        engine.publish(MetricCategory::ZkpV7, "m".to_string(), 2.0, 5);
        let events = engine.get_events("s1", 1);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].value, 1.0);
    }

    #[test]
    fn test_window_stats() {
        let mut engine = StreamEngine::new();
        engine.publish(MetricCategory::ZkpV7, "proofs".to_string(), 10.0, 5);
        engine.publish(MetricCategory::ZkpV7, "proofs".to_string(), 20.0, 5);
        engine.publish(MetricCategory::ZkpV7, "proofs".to_string(), 30.0, 5);
        let stats = engine.get_window_stats(&MetricCategory::ZkpV7).unwrap();
        assert_eq!(stats.count, 3);
        assert_eq!(stats.min, 10.0);
        assert_eq!(stats.max, 30.0);
        assert_eq!(stats.avg, 20.0);
    }

    #[test]
    fn test_history() {
        let mut engine = StreamEngine::new();
        engine.subscribe(make_subscriber("s1", 100, vec![])).unwrap();
        engine.publish(MetricCategory::ZkpV7, "m".to_string(), 1.0, 5);
        engine.publish(MetricCategory::ZkpV7, "m".to_string(), 2.0, 5);
        let history = engine.get_history(10);
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_reset_stats() {
        let mut engine = StreamEngine::new();
        engine.subscribe(make_subscriber("s1", 100, vec![])).unwrap();
        engine.publish(MetricCategory::ZkpV7, "m".to_string(), 1.0, 5);
        engine.reset_stats();
        assert_eq!(engine.stats.events_published, 0);
    }

    #[test]
    fn test_priority_ordering() {
        let mut engine = StreamEngine::new();
        engine.subscribe(make_subscriber("s1", 100, vec![])).unwrap();
        engine.publish(MetricCategory::ZkpV7, "low".to_string(), 1.0, 5);
        engine.publish(MetricCategory::ZkpV7, "high".to_string(), 2.0, 200);
        assert_eq!(engine.stats.queue_depth, 2);
    }

    #[test]
    fn test_process_queue() {
        let mut engine = StreamEngine::new();
        engine.config.priority_ordering = true;
        engine.subscribe(make_subscriber("s1", 100, vec![])).unwrap();
        engine.publish(MetricCategory::ZkpV7, "m".to_string(), 1.0, 10);
        let processed = engine.process_queue();
        assert_eq!(processed, 1);
    }

    #[test]
    fn test_active_subscriber_count() {
        let mut engine = StreamEngine::new();
        engine.subscribe(make_subscriber("s1", 100, vec![])).unwrap();
        assert_eq!(engine.active_subscriber_count(), 1);
    }

    #[test]
    fn test_paused_subscriber_count() {
        let mut engine = StreamEngine::new();
        let config = SubscriberConfig {
            id: "s1".to_string(),
            buffer_size: 3,
            categories: vec![],
            pause_threshold: 0.3,
            resume_threshold: 0.1,
        };
        engine.subscribe(config).unwrap();
        engine.publish(MetricCategory::ZkpV7, "m".to_string(), 1.0, 5);
        assert_eq!(engine.paused_subscriber_count(), 1);
    }

    #[test]
    fn test_clear_history() {
        let mut engine = StreamEngine::new();
        engine.subscribe(make_subscriber("s1", 100, vec![])).unwrap();
        engine.publish(MetricCategory::ZkpV7, "m".to_string(), 1.0, 5);
        engine.clear_history();
        assert_eq!(engine.get_history(10).len(), 0);
    }

    #[test]
    fn test_clear_windows() {
        let mut engine = StreamEngine::new();
        engine.publish(MetricCategory::ZkpV7, "m".to_string(), 1.0, 5);
        engine.clear_windows();
        assert!(engine.get_window_stats(&MetricCategory::ZkpV7).is_none());
    }

    #[test]
    fn test_window_stats_empty() {
        let stats = WindowStats::new();
        assert!(stats.is_empty());
    }

    #[test]
    fn test_window_stats_record() {
        let mut stats = WindowStats::new();
        stats.record(10.0, 1000);
        stats.record(20.0, 2000);
        assert_eq!(stats.count, 2);
        assert_eq!(stats.avg, 15.0);
    }

    #[test]
    fn test_subscriber_state_display() {
        assert_eq!(format!("{}", SubscriberState::Active), "active");
        assert_eq!(format!("{}", SubscriberState::Paused), "paused");
        assert_eq!(format!("{}", SubscriberState::Dropped), "dropped");
    }

    #[test]
    fn test_category_display() {
        assert_eq!(format!("{}", MetricCategory::ZkpV7), "zkp_v7");
        assert_eq!(format!("{}", MetricCategory::CrossPool), "cross_pool");
        assert_eq!(format!("{}", MetricCategory::Custom("test".to_string())), "custom:test");
    }

    #[test]
    fn test_error_display() {
        match StreamError::SubscriberNotFound("x".to_string()) {
            e => assert!(format!("{}", e).contains("x")),
        }
    }

    #[test]
    fn test_config_default() {
        let config = StreamConfig::default();
        assert_eq!(config.max_subscribers, 64);
        assert_eq!(config.default_buffer_size, 256);
    }

    #[test]
    fn test_stats_default() {
        let stats = StreamStats::default();
        assert_eq!(stats.events_published, 0);
    }

    #[test]
    fn test_stats_reset() {
        let mut stats = StreamStats::new();
        stats.events_published = 100;
        stats.reset();
        assert_eq!(stats.events_published, 0);
    }

    #[test]
    fn test_engine_default() {
        let engine = StreamEngine::default();
        assert_eq!(engine.subscribers.len(), 0);
    }

    #[test]
    fn test_get_config() {
        let engine = StreamEngine::new();
        assert_eq!(engine.get_config().max_subscribers, 64);
    }

    #[test]
    fn test_get_stats() {
        let engine = StreamEngine::new();
        assert_eq!(engine.get_stats().events_published, 0);
    }

    #[test]
    fn test_stream_event_new() {
        let event = StreamEvent::new(1, MetricCategory::ZkpV7, "test".to_string(), 42.0, 10);
        assert_eq!(event.sequence, 1);
        assert_eq!(event.value, 42.0);
        assert_eq!(event.priority, 10);
    }

    #[test]
    fn test_subscriber_buffer_usage() {
        let mut sub = Subscriber::new(make_subscriber("s1", 10, vec![]));
        assert_eq!(sub.buffer_usage(), 0.0);
        sub.enqueue(StreamEvent::new(1, MetricCategory::ZkpV7, "m".to_string(), 1.0, 5)).unwrap();
        assert_eq!(sub.buffer_usage(), 0.1);
    }

    #[test]
    fn test_subscriber_drain() {
        let mut sub = Subscriber::new(make_subscriber("s1", 100, vec![]));
        sub.enqueue(StreamEvent::new(1, MetricCategory::ZkpV7, "m".to_string(), 1.0, 5)).unwrap();
        sub.enqueue(StreamEvent::new(2, MetricCategory::ZkpV7, "m".to_string(), 2.0, 5)).unwrap();
        let events = sub.drain(1);
        assert_eq!(events.len(), 1);
        assert_eq!(sub.buffer.len(), 1);
    }

    #[test]
    fn test_multiple_subscribers_different_filters() {
        let mut engine = StreamEngine::new();
        engine.subscribe(make_subscriber("zkp", 100, vec![MetricCategory::ZkpV7])).unwrap();
        engine.subscribe(make_subscriber("sys", 100, vec![MetricCategory::System])).unwrap();
        engine.publish(MetricCategory::ZkpV7, "proofs".to_string(), 10.0, 5);
        engine.publish(MetricCategory::System, "cpu".to_string(), 50.0, 5);
        assert_eq!(engine.get_events("zkp", 10).len(), 1);
        assert_eq!(engine.get_events("sys", 10).len(), 1);
    }

    #[test]
    fn test_sequence_incrementing() {
        let mut engine = StreamEngine::new();
        engine.subscribe(make_subscriber("s1", 100, vec![])).unwrap();
        engine.publish(MetricCategory::ZkpV7, "m".to_string(), 1.0, 5);
        engine.publish(MetricCategory::ZkpV7, "m".to_string(), 2.0, 5);
        engine.publish(MetricCategory::ZkpV7, "m".to_string(), 3.0, 5);
        let events = engine.get_events("s1", 10);
        assert_eq!(events[0].sequence, 1);
        assert_eq!(events[1].sequence, 2);
        assert_eq!(events[2].sequence, 3);
    }

    #[test]
    fn test_get_events_missing_subscriber() {
        let mut engine = StreamEngine::new();
        let events = engine.get_events("nonexistent", 10);
        assert!(events.is_empty());
    }

    #[test]
    fn test_get_subscriber_state_missing() {
        let engine = StreamEngine::new();
        assert!(engine.get_subscriber_state("nonexistent").is_none());
    }
}
