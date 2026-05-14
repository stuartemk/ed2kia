//! Dynamic Router v4 — Latency-aware routing with reputation-weighted decisions.
//!
//! Provides dynamic routing decisions based on historical latency, current load,
//! and reputation v2 scores. Falls back to static Kademlia routing when confidence
//! drops below threshold.
//!
//! **Linux Analogy:** Like `iproute2` policy routing where traffic is steered
//! based on real-time metrics and historical performance data.
//!
//! Protected with `#[cfg(feature = "v1.5-sprint1")]`.

#[cfg(feature = "v1.5-sprint1")]
mod internal {
    use std::collections::{HashMap, VecDeque};

    // ─── Errors ───────────────────────────────────────────────────────────────────

    /// Errors for dynamic router operations.
    #[derive(Debug, Clone, PartialEq)]
    pub enum RouterError {
        /// Route not found.
        RouteNotFound(String),
        /// No viable routes available.
        NoRoutesAvailable,
        /// Confidence too low for adaptive routing.
        LowConfidence { confidence: f64, threshold: f64 },
        /// Invalid latency value.
        InvalidLatency(f64),
        /// Routing table full.
        TableFull,
    }

    impl std::fmt::Display for RouterError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::RouteNotFound(id) => write!(f, "Route not found: {}", id),
                Self::NoRoutesAvailable => write!(f, "No viable routes available"),
                Self::LowConfidence { confidence, threshold } => {
                    write!(f, "Confidence too low: {:.4} < {:.4}", confidence, threshold)
                }
                Self::InvalidLatency(ms) => write!(f, "Invalid latency: {}ms", ms),
                Self::TableFull => write!(f, "Routing table full"),
            }
        }
    }

    // ─── Config ───────────────────────────────────────────────────────────────────

    /// Configuration for dynamic router.
    #[derive(Debug, Clone)]
    pub struct RouterConfig {
        /// Maximum routes in table.
        pub max_routes: usize,
        /// Latency history window size.
        pub latency_window: usize,
        /// Minimum confidence for adaptive routing.
        pub min_confidence: f64,
        /// Latency weight in routing score.
        pub latency_weight: f64,
        /// Load weight in routing score.
        pub load_weight: f64,
        /// Reputation weight in routing score.
        pub reputation_weight: f64,
        /// EMA alpha for latency smoothing.
        pub latency_alpha: f64,
        /// Fallback to static routing on low confidence.
        pub fallback_to_static: bool,
    }

    impl Default for RouterConfig {
        fn default() -> Self {
            Self {
                max_routes: 256,
                latency_window: 50,
                min_confidence: 0.0,
                latency_weight: 0.4,
                load_weight: 0.3,
                reputation_weight: 0.3,
                latency_alpha: 0.3,
                fallback_to_static: true,
            }
        }
    }

    // ─── Load Metric ──────────────────────────────────────────────────────────────

    /// Load metric for a route target.
    #[derive(Debug, Clone)]
    pub struct LoadMetric {
        /// Current load factor (0.0-1.0).
        pub current_load: f64,
        /// EMA-smoothed load.
        pub ema_load: f64,
        /// Peak load in current window.
        pub peak_load: f64,
        /// Load samples.
        pub samples: VecDeque<f64>,
    }

    impl LoadMetric {
        pub fn new(max_samples: usize) -> Self {
            Self {
                current_load: 0.0,
                ema_load: 0.0,
                peak_load: 0.0,
                samples: VecDeque::with_capacity(max_samples),
            }
        }

        pub fn update(&mut self, load: f64, alpha: f64, max_samples: usize) {
            self.current_load = load;
            if self.ema_load == 0.0 {
                self.ema_load = load;
            } else {
                self.ema_load = alpha * load + (1.0 - alpha) * self.ema_load;
            }
            self.peak_load = self.peak_load.max(load);
            self.samples.push_back(load);
            while self.samples.len() > max_samples {
                self.samples.pop_front();
            }
        }

        pub fn avg_recent(&self) -> f64 {
            if self.samples.is_empty() {
                return 0.0;
            }
            self.samples.iter().sum::<f64>() / self.samples.len() as f64
        }
    }

    // ─── Dynamic Route Entry ──────────────────────────────────────────────────────

    /// A dynamic route entry with latency and reputation tracking.
    #[derive(Debug, Clone)]
    pub struct DynamicRouteEntry {
        /// Target identifier (node/pool ID).
        pub target_id: String,
        /// Historical latency samples (ms).
        pub latency_history: VecDeque<f64>,
        /// EMA-smoothed latency (ms).
        pub ema_latency: f64,
        /// Load metrics.
        pub load: LoadMetric,
        /// Reputation score (0.0-1.0).
        pub reputation: f64,
        /// Confidence in metrics (0.0-1.0).
        pub confidence: f64,
        /// Active flag.
        pub active: bool,
    }

    impl DynamicRouteEntry {
        pub fn new(target_id: String, reputation: f64, max_samples: usize) -> Self {
            Self {
                target_id,
                latency_history: VecDeque::with_capacity(max_samples),
                ema_latency: 0.0,
                load: LoadMetric::new(max_samples),
                reputation,
                confidence: 0.0,
                active: true,
            }
        }

        /// Record a latency observation.
        pub fn record_latency(&mut self, latency_ms: f64, alpha: f64, max_samples: usize) {
            if latency_ms < 0.0 {
                return;
            }
            if self.ema_latency == 0.0 {
                self.ema_latency = latency_ms;
            } else {
                self.ema_latency = alpha * latency_ms + (1.0 - alpha) * self.ema_latency;
            }
            self.latency_history.push_back(latency_ms);
            while self.latency_history.len() > max_samples {
                self.latency_history.pop_front();
            }
            // Update confidence based on sample depth
            let depth = self.latency_history.len() as f64;
            self.confidence = (depth / max_samples as f64).min(1.0);
        }

        /// Compute routing score (lower is better).
        pub fn routing_score(&self, latency_weight: f64, load_weight: f64, reputation_weight: f64) -> f64 {
            // Normalize latency to 0-1 range (assume max 1000ms)
            let norm_latency = (self.ema_latency / 1000.0).min(1.0);
            norm_latency * latency_weight
                + self.load.ema_load * load_weight
                + (1.0 - self.reputation) * reputation_weight
        }

        /// Check if this route meets confidence threshold.
        pub fn meets_confidence(&self, threshold: f64) -> bool {
            self.confidence >= threshold && self.active
        }
    }

    // ─── Routing Decision ─────────────────────────────────────────────────────────

    /// Result of a routing decision.
    #[derive(Debug, Clone)]
    pub struct RoutingDecision {
        /// Selected target ID.
        pub target_id: String,
        /// Decision confidence.
        pub confidence: f64,
        /// Estimated latency (ms).
        pub estimated_latency_ms: f64,
        /// Whether static fallback was used.
        pub used_fallback: bool,
        /// Routing score of selected target.
        pub score: f64,
    }

    // ─── Router Stats ─────────────────────────────────────────────────────────────

    /// Statistics for routing decisions.
    #[derive(Debug, Clone)]
    pub struct RouterStats {
        pub total_decisions: usize,
        pub total_fallbacks: usize,
        pub avg_decision_time_ms: f64,
        pub avg_confidence: f64,
        pub total_rejections: usize,
    }

    impl Default for RouterStats {
        fn default() -> Self {
            Self {
                total_decisions: 0,
                total_fallbacks: 0,
                avg_decision_time_ms: 0.0,
                avg_confidence: 1.0,
                total_rejections: 0,
            }
        }
    }

    impl RouterStats {
        pub fn record_decision(&mut self, time_ms: f64, confidence: f64) {
            self.total_decisions += 1;
            self.avg_decision_time_ms =
                (self.avg_decision_time_ms * (self.total_decisions - 1) as f64 + time_ms)
                    / self.total_decisions as f64;
            self.avg_confidence =
                (self.avg_confidence * (self.total_decisions - 1) as f64 + confidence)
                    / self.total_decisions as f64;
        }

        pub fn record_fallback(&mut self) {
            self.total_fallbacks += 1;
        }

        pub fn record_rejection(&mut self) {
            self.total_rejections += 1;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // ─── Dynamic Router ───────────────────────────────────────────────────────────

    /// Dynamic router with latency-aware, reputation-weighted decisions.
    pub struct DynamicRouter {
        config: RouterConfig,
        routes: HashMap<String, DynamicRouteEntry>,
        pub stats: RouterStats,
    }

    impl DynamicRouter {
        pub fn new(config: RouterConfig) -> Self {
            Self {
                config,
                routes: HashMap::new(),
                stats: RouterStats::default(),
            }
        }

        /// Register a new route target.
        pub fn register_route(&mut self, target_id: String, reputation: f64) -> Result<(), RouterError> {
            if self.routes.len() >= self.config.max_routes {
                return Err(RouterError::TableFull);
            }
            if self.routes.contains_key(&target_id) {
                return Err(RouterError::RouteNotFound(target_id)); // already exists
            }
            let entry = DynamicRouteEntry::new(target_id.clone(), reputation, self.config.latency_window);
            self.routes.insert(target_id, entry);
            Ok(())
        }

        /// Record latency observation for a target.
        pub fn record_latency(&mut self, target_id: &str, latency_ms: f64) -> Result<(), RouterError> {
            let entry = self.routes.get_mut(target_id)
                .ok_or(RouterError::RouteNotFound(target_id.to_string()))?;
            if latency_ms < 0.0 {
                return Err(RouterError::InvalidLatency(latency_ms));
            }
            entry.record_latency(latency_ms, self.config.latency_alpha, self.config.latency_window);
            Ok(())
        }

        /// Update load for a target.
        pub fn update_load(&mut self, target_id: &str, load: f64) -> Result<(), RouterError> {
            let entry = self.routes.get_mut(target_id)
                .ok_or(RouterError::RouteNotFound(target_id.to_string()))?;
            entry.load.update(load, self.config.latency_alpha, self.config.latency_window);
            Ok(())
        }

        /// Make a routing decision.
        pub fn decide(&mut self) -> Result<RoutingDecision, RouterError> {
            let start = std::time::Instant::now();

            // Find viable routes
            let viable: Vec<_> = self.routes.values()
                .filter(|r| r.active)
                .collect();

            if viable.is_empty() {
                return Err(RouterError::NoRoutesAvailable);
            }

            // Check confidence
            let confident_routes: Vec<_> = viable.iter()
                .filter(|r| r.meets_confidence(self.config.min_confidence))
                .collect();

            let selected = if !confident_routes.is_empty() {
                // Pick best route by score
                confident_routes.iter()
                    .min_by(|a, b| {
                        a.routing_score(
                            self.config.latency_weight,
                            self.config.load_weight,
                            self.config.reputation_weight,
                        ).partial_cmp(
                            &b.routing_score(
                                self.config.latency_weight,
                                self.config.load_weight,
                                self.config.reputation_weight,
                            )
                        ).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .ok_or(RouterError::NoRoutesAvailable)?
            } else if self.config.fallback_to_static {
                // Fallback: pick by reputation
                self.stats.record_fallback();
                viable.iter()
                    .max_by(|a, b| {
                        a.reputation.partial_cmp(&b.reputation)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .ok_or(RouterError::NoRoutesAvailable)?
            } else {
                return Err(RouterError::LowConfidence {
                    confidence: 0.0,
                    threshold: self.config.min_confidence,
                });
            };

            let score = selected.routing_score(
                self.config.latency_weight,
                self.config.load_weight,
                self.config.reputation_weight,
            );
            let used_fallback = confident_routes.is_empty();

            if used_fallback {
                self.stats.record_fallback();
            }

            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            self.stats.record_decision(elapsed, selected.confidence);

            Ok(RoutingDecision {
                target_id: selected.target_id.clone(),
                confidence: selected.confidence,
                estimated_latency_ms: selected.ema_latency,
                used_fallback,
                score,
            })
        }

        /// Get route entry.
        pub fn get_route(&self, target_id: &str) -> Option<&DynamicRouteEntry> {
            self.routes.get(target_id)
        }

        /// Deactivate a route.
        pub fn deactivate_route(&mut self, target_id: &str) -> Result<(), RouterError> {
            let entry = self.routes.get_mut(target_id)
                .ok_or(RouterError::RouteNotFound(target_id.to_string()))?;
            entry.active = false;
            Ok(())
        }

        /// Activate a route.
        pub fn activate_route(&mut self, target_id: &str) -> Result<(), RouterError> {
            let entry = self.routes.get_mut(target_id)
                .ok_or(RouterError::RouteNotFound(target_id.to_string()))?;
            entry.active = true;
            Ok(())
        }

        /// Count active routes.
        pub fn active_route_count(&self) -> usize {
            self.routes.values().filter(|r| r.active).count()
        }

        /// Reset statistics.
        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }
    }

    impl Default for DynamicRouter {
        fn default() -> Self {
            Self::new(RouterConfig::default())
        }
    }

    // ─── Tests ────────────────────────────────────────────────────────────────────

    #[cfg(test)]
    mod tests {
        use super::*;

        fn make_config() -> RouterConfig {
            RouterConfig {
                max_routes: 32,
                latency_window: 20,
                min_confidence: 0.5,
                latency_weight: 0.4,
                load_weight: 0.3,
                reputation_weight: 0.3,
                latency_alpha: 0.3,
                fallback_to_static: true,
            }
        }

        #[test]
        fn test_router_creation() {
            let router = DynamicRouter::default();
            assert_eq!(router.active_route_count(), 0);
        }

        #[test]
        fn test_router_with_config() {
            let config = make_config();
            let router = DynamicRouter::new(config);
            assert_eq!(router.config.max_routes, 32);
        }

        #[test]
        fn test_register_route() {
            let mut router = DynamicRouter::default();
            router.register_route("n1".to_string(), 0.9).unwrap();
            assert_eq!(router.active_route_count(), 1);
        }

        #[test]
        fn test_register_route_duplicate() {
            let mut router = DynamicRouter::default();
            router.register_route("n1".to_string(), 0.9).unwrap();
            let result = router.register_route("n1".to_string(), 0.8);
            assert!(result.is_err());
        }

        #[test]
        fn test_register_route_table_full() {
            let config = RouterConfig { max_routes: 2, ..Default::default() };
            let mut router = DynamicRouter::new(config);
            router.register_route("n1".to_string(), 0.9).unwrap();
            router.register_route("n2".to_string(), 0.8).unwrap();
            assert!(router.register_route("n3".to_string(), 0.7).is_err());
        }

        #[test]
        fn test_record_latency() {
            let mut router = DynamicRouter::default();
            router.register_route("n1".to_string(), 0.9).unwrap();
            router.record_latency("n1", 50.0).unwrap();
            let entry = router.get_route("n1").unwrap();
            assert_eq!(entry.ema_latency, 50.0);
        }

        #[test]
        fn test_record_latency_invalid() {
            let mut router = DynamicRouter::default();
            router.register_route("n1".to_string(), 0.9).unwrap();
            let result = router.record_latency("n1", -1.0);
            assert!(result.is_err());
        }

        #[test]
        fn test_update_load() {
            let mut router = DynamicRouter::default();
            router.register_route("n1".to_string(), 0.9).unwrap();
            router.update_load("n1", 0.5).unwrap();
            let entry = router.get_route("n1").unwrap();
            assert!(entry.load.ema_load > 0.0);
        }

        #[test]
        fn test_decide_single_route() {
            let mut router = DynamicRouter::default();
            router.register_route("n1".to_string(), 0.9).unwrap();
            for _ in 0..20 {
                router.record_latency("n1", 50.0).unwrap();
            }
            let decision = router.decide().unwrap();
            assert_eq!(decision.target_id, "n1");
        }

        #[test]
        fn test_decide_selects_best() {
            let mut router = DynamicRouter::default();
            router.register_route("fast".to_string(), 0.9).unwrap();
            router.register_route("slow".to_string(), 0.9).unwrap();
            for _ in 0..20 {
                router.record_latency("fast", 10.0).unwrap();
                router.record_latency("slow", 200.0).unwrap();
            }
            let decision = router.decide().unwrap();
            assert_eq!(decision.target_id, "fast");
        }

        #[test]
        fn test_decide_fallback() {
            let config = RouterConfig {
                min_confidence: 0.9,
                ..RouterConfig::default()
            };
            let mut router = DynamicRouter::new(config);
            router.register_route("n1".to_string(), 0.9).unwrap();
            router.register_route("n2".to_string(), 0.5).unwrap();
            // No latency data — confidence low
            let decision = router.decide().unwrap();
            assert!(decision.used_fallback);
        }

        #[test]
        fn test_decide_no_routes() {
            let mut router = DynamicRouter::default();
            let result = router.decide();
            assert!(matches!(result, Err(RouterError::NoRoutesAvailable)));
        }

        #[test]
        fn test_deactivate_route() {
            let mut router = DynamicRouter::default();
            router.register_route("n1".to_string(), 0.9).unwrap();
            router.deactivate_route("n1").unwrap();
            assert_eq!(router.active_route_count(), 0);
        }

        #[test]
        fn test_activate_route() {
            let mut router = DynamicRouter::default();
            router.register_route("n1".to_string(), 0.9).unwrap();
            router.deactivate_route("n1").unwrap();
            router.activate_route("n1").unwrap();
            assert_eq!(router.active_route_count(), 1);
        }

        #[test]
        fn test_confidence_builds() {
            let mut entry = DynamicRouteEntry::new("test".to_string(), 0.9, 20);
            assert!(entry.confidence == 0.0);
            entry.record_latency(50.0, 0.3, 20);
            assert!(entry.confidence > 0.0);
            assert!(entry.confidence < 1.0);
            for _ in 0..20 {
                entry.record_latency(50.0, 0.3, 20);
            }
            assert!((entry.confidence - 1.0).abs() < 0.01);
        }

        #[test]
        fn test_ema_latency() {
            let mut entry = DynamicRouteEntry::new("test".to_string(), 0.9, 20);
            entry.record_latency(100.0, 0.5, 20);
            entry.record_latency(50.0, 0.5, 20);
            assert!((entry.ema_latency - 75.0).abs() < 0.01);
        }

        #[test]
        fn test_load_metric_avg() {
            let mut metric = LoadMetric::new(10);
            metric.update(0.2, 0.5, 10);
            metric.update(0.4, 0.5, 10);
            metric.update(0.6, 0.5, 10);
            let avg = metric.avg_recent();
            assert!(avg > 0.0);
        }

        #[test]
        fn test_load_metric_peak() {
            let mut metric = LoadMetric::new(10);
            metric.update(0.3, 0.5, 10);
            metric.update(0.8, 0.5, 10);
            metric.update(0.1, 0.5, 10);
            assert_eq!(metric.peak_load, 0.8);
        }

        #[test]
        fn test_routing_score() {
            let mut entry = DynamicRouteEntry::new("test".to_string(), 0.9, 20);
            entry.record_latency(100.0, 0.5, 20);
            let score = entry.routing_score(0.4, 0.3, 0.3);
            assert!(score >= 0.0);
            assert!(score <= 1.0);
        }

        #[test]
        fn test_stats_tracking() {
            let mut router = DynamicRouter::default();
            router.register_route("n1".to_string(), 0.9).unwrap();
            for _ in 0..20 {
                router.record_latency("n1", 50.0).unwrap();
            }
            router.decide().unwrap();
            assert_eq!(router.stats.total_decisions, 1);
        }

        #[test]
        fn test_reset_stats() {
            let mut router = DynamicRouter::default();
            router.register_route("n1".to_string(), 0.9).unwrap();
            for _ in 0..20 {
                router.record_latency("n1", 50.0).unwrap();
            }
            router.decide().unwrap();
            router.reset_stats();
            assert_eq!(router.stats.total_decisions, 0);
        }

        #[test]
        fn test_config_default() {
            let config = RouterConfig::default();
            assert_eq!(config.min_confidence, 0.0);
            assert!(config.fallback_to_static);
        }

        #[test]
        fn test_stats_default() {
            let stats = RouterStats::default();
            assert_eq!(stats.total_decisions, 0);
        }

        #[test]
        fn test_error_display() {
            let e = RouterError::RouteNotFound("x".to_string());
            assert!(format!("{}", e).contains("x"));
        }

        #[test]
        fn test_route_new() {
            let entry = DynamicRouteEntry::new("n1".to_string(), 0.9, 20);
            assert!(entry.active);
            assert_eq!(entry.reputation, 0.9);
        }

        #[test]
        fn test_router_default() {
            let router = DynamicRouter::default();
            assert_eq!(router.routes.len(), 0);
        }

        #[test]
        fn test_reputation_fallback() {
            let config = RouterConfig {
                min_confidence: 0.9,
                ..RouterConfig::default()
            };
            let mut router = DynamicRouter::new(config);
            router.register_route("high_rep".to_string(), 0.95).unwrap();
            router.register_route("low_rep".to_string(), 0.3).unwrap();
            // No latency data — confidence 0.0 < 0.9 — should fall back to reputation
            let decision = router.decide().unwrap();
            assert_eq!(decision.target_id, "high_rep");
            assert!(decision.used_fallback);
        }

        #[test]
        fn test_decision_timing() {
            let mut router = DynamicRouter::default();
            for i in 0..10 {
                router.register_route(format!("n{}", i), 0.9).unwrap();
            }
            for i in 0..10 {
                for _ in 0..20 {
                    router.record_latency(&format!("n{}", i), 50.0 + i as f64).unwrap();
                }
            }
            let decision = router.decide().unwrap();
            assert!(decision.estimated_latency_ms > 0.0);
            assert!(router.stats.avg_decision_time_ms >= 0.0);
        }
    }
}

#[cfg(feature = "v1.5-sprint1")]
pub use internal::*;
