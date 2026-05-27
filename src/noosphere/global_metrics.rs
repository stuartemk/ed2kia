//! Noospheric Global Metrics — Sprint 58
//!
//! Mathematical metrics for measuring the health and intelligence amplification
//! of the global Noosphere network.
//!
//! **Core Equations:**
//!
//! **Noospheric Health (NH):**
//! ```text
//! NH(t) = α · E(t) + β · M(t) + γ · A(t)
//!
//! Where:
//!   E(t) = Global Ethical Coherence = mean(Z_SCT) across all active nodes
//!   M(t) = Constructive Emergence Rate = macro_concepts_born / time_window
//!   A(t) = Attractor Basin Stability = 1.0 - lyapunov_variance
//!   α + β + γ = 1.0 (default: α=0.4, β=0.3, γ=0.3)
//! ```
//!
//! **Symbiotic Intelligence Amplification (SIA):**
//! ```text
//! SIA(t) = (R_human(t) + R_network(t)) / R_human(t)
//!
//! Where:
//!   R_human(t) = Human problem resolution rate (baseline)
//!   R_network(t) = Network-assisted problem resolution rate
//!   SIA ≥ 1.0 indicates the network amplifies human intelligence
//! ```
//!
//! **Feature Gate:** `v4.0-snap-activation`

/// Errors specific to global metric computation.
#[derive(Debug, Clone, PartialEq)]
pub enum MetricsError {
    /// Insufficient data for computation.
    InsufficientData(String),
    /// Invalid metric configuration.
    InvalidConfig(String),
    /// Division by zero in ratio computation.
    DivisionByZero,
    /// Negative value where positive expected.
    NegativeValue(String),
}

impl std::fmt::Display for MetricsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricsError::InsufficientData(msg) => write!(f, "Insufficient data: {}", msg),
            MetricsError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
            MetricsError::DivisionByZero => write!(f, "Division by zero in metric computation"),
            MetricsError::NegativeValue(msg) => write!(f, "Negative value where positive expected: {}", msg),
        }
    }
}

/// Configuration for global metric weights and thresholds.
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Weight for Ethical Coherence in NH (default: 0.4).
    pub weight_ethical: f64,
    /// Weight for Emergence Rate in NH (default: 0.3).
    pub weight_emergence: f64,
    /// Weight for Attractor Stability in NH (default: 0.3).
    pub weight_attractor: f64,
    /// Minimum NH for healthy network (default: 0.6).
    pub min_healthy_threshold: f64,
    /// Minimum NH before quarantine consideration (default: 0.3).
    pub quarantine_threshold: f64,
    /// Minimum NH before apoptosis consideration (default: 0.1).
    pub apoptosis_threshold: f64,
    /// Time window for emergence rate (in ticks, default: 100).
    pub emergence_window: u32,
    /// Baseline human resolution rate for SIA (default: 1.0).
    pub human_baseline_rate: f64,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            weight_ethical: 0.4,
            weight_emergence: 0.3,
            weight_attractor: 0.3,
            min_healthy_threshold: 0.6,
            quarantine_threshold: 0.3,
            apoptosis_threshold: 0.1,
            emergence_window: 100,
            human_baseline_rate: 1.0,
        }
    }
}

impl MetricsConfig {
    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<(), MetricsError> {
        let sum = self.weight_ethical + self.weight_emergence + self.weight_attractor;
        if (sum - 1.0).abs() > 0.01 {
            return Err(MetricsError::InvalidConfig(
                "Weights must sum to 1.0".to_string(),
            ));
        }
        if self.weight_ethical < 0.0 || self.weight_emergence < 0.0 || self.weight_attractor < 0.0 {
            return Err(MetricsError::InvalidConfig(
                "Weights must be non-negative".to_string(),
            ));
        }
        if self.emergence_window == 0 {
            return Err(MetricsError::InvalidConfig(
                "Emergence window must be > 0".to_string(),
            ));
        }
        if self.human_baseline_rate <= 0.0 {
            return Err(MetricsError::InvalidConfig(
                "Human baseline rate must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

/// Snapshot of network state for metric computation.
#[derive(Debug, Clone)]
pub struct NetworkSnapshot {
    /// Mean Z-score from SCT across all active nodes.
    pub mean_z_sct: f64,
    /// Number of macro-concepts born in the emergence window.
    pub macro_concepts_born: u32,
    /// Variance of Lyapunov exponents across the network.
    pub lyapunov_variance: f64,
    /// Network-assisted problem resolution rate.
    pub network_resolution_rate: f64,
    /// Total active nodes.
    pub active_nodes: usize,
    /// Tick number.
    pub tick: u64,
}

/// Computed Noospheric Health score with component breakdown.
#[derive(Debug, Clone)]
pub struct HealthScore {
    /// Composite Noospheric Health value [0, 1].
    pub nh: f64,
    /// Ethical Coherence component [0, 1].
    pub ethical_coherence: f64,
    /// Emergence Rate component [0, 1].
    pub emergence_rate: f64,
    /// Attractor Stability component [0, 1].
    pub attractor_stability: f64,
    /// Tick number when computed.
    pub tick: u64,
}

/// Computed Symbiotic Intelligence Amplification score.
#[derive(Debug, Clone)]
pub struct SiaScore {
    /// SIA ratio (≥ 1.0 means amplification).
    pub sia: f64,
    /// Human baseline resolution rate.
    pub human_rate: f64,
    /// Network-assisted resolution rate.
    pub network_rate: f64,
    /// Tick number when computed.
    pub tick: u64,
}

/// Global Metrics Engine — Computes NH and SIA from network snapshots.
#[derive(Debug)]
pub struct GlobalMetrics {
    config: MetricsConfig,
    history: Vec<HealthScore>,
    sia_history: Vec<SiaScore>,
    max_history: usize,
}

impl GlobalMetrics {
    /// Create with default configuration.
    pub fn new() -> Self {
        Self {
            config: MetricsConfig::default(),
            history: Vec::new(),
            sia_history: Vec::new(),
            max_history: 1000,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: MetricsConfig) -> Result<Self, MetricsError> {
        config.validate()?;
        Ok(Self {
            config,
            history: Vec::new(),
            sia_history: Vec::new(),
            max_history: 1000,
        })
    }

    /// Compute Noospheric Health from a network snapshot.
    ///
    /// NH(t) = α · E(t) + β · M(t) + γ · A(t)
    pub fn compute_health(&mut self, snapshot: &NetworkSnapshot) -> Result<HealthScore, MetricsError> {
        // E(t): Global Ethical Coherence — clamp mean Z to [0, 1]
        let ethical_coherence = snapshot.mean_z_sct.clamp(0.0, 1.0);

        // M(t): Constructive Emergence Rate — normalize by window
        let emergence_rate = (snapshot.macro_concepts_born as f64 / self.config.emergence_window as f64)
            .tanh()
            .clamp(0.0, 1.0);

        // A(t): Attractor Basin Stability — low variance = high stability
        let attractor_stability = (1.0 - snapshot.lyapunov_variance.clamp(0.0, 1.0)).clamp(0.0, 1.0);

        // Composite NH
        let nh = self.config.weight_ethical * ethical_coherence
            + self.config.weight_emergence * emergence_rate
            + self.config.weight_attractor * attractor_stability;

        let score = HealthScore {
            nh: nh.clamp(0.0, 1.0),
            ethical_coherence,
            emergence_rate,
            attractor_stability,
            tick: snapshot.tick,
        };

        self.history.push(score.clone());
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        Ok(score)
    }

    /// Compute Symbiotic Intelligence Amplification from a network snapshot.
    ///
    /// SIA(t) = (R_human(t) + R_network(t)) / R_human(t)
    pub fn compute_sia(&mut self, snapshot: &NetworkSnapshot) -> Result<SiaScore, MetricsError> {
        if self.config.human_baseline_rate <= 0.0 {
            return Err(MetricsError::DivisionByZero);
        }

        let sia = (self.config.human_baseline_rate + snapshot.network_resolution_rate)
            / self.config.human_baseline_rate;

        let score = SiaScore {
            sia: sia.max(1.0), // SIA is always ≥ 1.0 by definition
            human_rate: self.config.human_baseline_rate,
            network_rate: snapshot.network_resolution_rate,
            tick: snapshot.tick,
        };

        self.sia_history.push(score.clone());
        if self.sia_history.len() > self.max_history {
            self.sia_history.remove(0);
        }

        Ok(score)
    }

    /// Compute both NH and SIA from a single snapshot.
    pub fn compute_all(
        &mut self,
        snapshot: &NetworkSnapshot,
    ) -> Result<(HealthScore, SiaScore), MetricsError> {
        let health = self.compute_health(snapshot)?;
        let sia = self.compute_sia(snapshot)?;
        Ok((health, sia))
    }

    /// Check if the network is healthy based on current NH.
    pub fn is_healthy(&self, nh: f64) -> bool {
        nh >= self.config.min_healthy_threshold
    }

    /// Check if quarantine should be considered.
    pub fn needs_quarantine(&self, nh: f64) -> bool {
        nh < self.config.quarantine_threshold
    }

    /// Check if apoptosis should be considered.
    pub fn needs_apoptosis(&self, nh: f64) -> bool {
        nh < self.config.apoptosis_threshold
    }

    /// Get the latest health score.
    pub fn latest_health(&self) -> Option<&HealthScore> {
        self.history.last()
    }

    /// Get the latest SIA score.
    pub fn latest_sia(&self) -> Option<&SiaScore> {
        self.sia_history.last()
    }

    /// Get average NH over the last N scores.
    pub fn average_health(&self, last_n: usize) -> Option<f64> {
        let len = self.history.len().min(last_n);
        if len == 0 {
            return None;
        }
        let sum: f64 = self.history.iter().rev().take(len).map(|s| s.nh).sum();
        Some(sum / len as f64)
    }

    /// Get average SIA over the last N scores.
    pub fn average_sia(&self, last_n: usize) -> Option<f64> {
        let len = self.sia_history.len().min(last_n);
        if len == 0 {
            return None;
        }
        let sum: f64 = self.sia_history.iter().rev().take(len).map(|s| s.sia).sum();
        Some(sum / len as f64)
    }

    /// Get NH trend (positive = improving, negative = declining).
    pub fn health_trend(&self, window: usize) -> Option<f64> {
        if self.history.len() < 2 {
            return None;
        }
        let len = self.history.len().min(window);
        let recent: &[HealthScore] = &self.history[self.history.len() - len..];
        if recent.len() < 2 {
            return None;
        }
        Some(recent.last().unwrap().nh - recent.first().unwrap().nh)
    }

    /// Reset all history.
    pub fn reset(&mut self) {
        self.history.clear();
        self.sia_history.clear();
    }

    /// Update configuration.
    pub fn update_config(&mut self, config: MetricsConfig) -> Result<(), MetricsError> {
        config.validate()?;
        self.config = config;
        Ok(())
    }
}

impl Default for GlobalMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_snapshot(tick: u64) -> NetworkSnapshot {
        NetworkSnapshot {
            mean_z_sct: 0.8,
            macro_concepts_born: 10,
            lyapunov_variance: 0.1,
            network_resolution_rate: 2.0,
            active_nodes: 1000,
            tick,
        }
    }

    #[test]
    fn test_metrics_creation() {
        let m = GlobalMetrics::new();
        assert!(m.latest_health().is_none());
    }

    #[test]
    fn test_metrics_custom_config() {
        let config = MetricsConfig {
            weight_ethical: 0.5,
            weight_emergence: 0.25,
            weight_attractor: 0.25,
            ..Default::default()
        };
        let m = GlobalMetrics::with_config(config).unwrap();
        assert_eq!(m.config.weight_ethical, 0.5);
    }

    #[test]
    fn test_invalid_config_weights() {
        let config = MetricsConfig {
            weight_ethical: 0.5,
            weight_emergence: 0.5,
            weight_attractor: 0.5,
            ..Default::default()
        };
        assert!(GlobalMetrics::with_config(config).is_err());
    }

    #[test]
    fn test_invalid_config_zero_window() {
        let config = MetricsConfig {
            emergence_window: 0,
            ..Default::default()
        };
        assert!(GlobalMetrics::with_config(config).is_err());
    }

    #[test]
    fn test_compute_health() {
        let mut m = GlobalMetrics::new();
        let snapshot = valid_snapshot(1);
        let score = m.compute_health(&snapshot).unwrap();
        assert!(score.nh > 0.0);
        assert!(score.nh <= 1.0);
        assert_eq!(score.tick, 1);
    }

    #[test]
    fn test_compute_health_high_ethical() {
        let mut m = GlobalMetrics::new();
        let snapshot = NetworkSnapshot {
            mean_z_sct: 1.0,
            macro_concepts_born: 50,
            lyapunov_variance: 0.0,
            network_resolution_rate: 5.0,
            active_nodes: 10_000,
            tick: 1,
        };
        let score = m.compute_health(&snapshot).unwrap();
        assert!(score.ethical_coherence == 1.0);
        assert!(score.attractor_stability == 1.0);
        assert!(score.nh > 0.8);
    }

    #[test]
    fn test_compute_health_low_ethical() {
        let mut m = GlobalMetrics::new();
        let snapshot = NetworkSnapshot {
            mean_z_sct: -0.5,
            macro_concepts_born: 0,
            lyapunov_variance: 1.0,
            network_resolution_rate: 0.0,
            active_nodes: 100,
            tick: 1,
        };
        let score = m.compute_health(&snapshot).unwrap();
        assert!(score.ethical_coherence == 0.0);
        assert!(score.emergence_rate == 0.0);
        assert!(score.attractor_stability == 0.0);
        assert!(score.nh == 0.0);
    }

    #[test]
    fn test_compute_sia() {
        let mut m = GlobalMetrics::new();
        let snapshot = valid_snapshot(1);
        let score = m.compute_sia(&snapshot).unwrap();
        // SIA = (1.0 + 2.0) / 1.0 = 3.0
        assert!((score.sia - 3.0).abs() < 0.001);
        assert!(score.sia >= 1.0);
    }

    #[test]
    fn test_compute_sia_minimum() {
        let mut m = GlobalMetrics::new();
        let snapshot = NetworkSnapshot {
            network_resolution_rate: 0.0,
            ..valid_snapshot(1)
        };
        let score = m.compute_sia(&snapshot).unwrap();
        // SIA = (1.0 + 0.0) / 1.0 = 1.0
        assert!((score.sia - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_compute_all() {
        let mut m = GlobalMetrics::new();
        let snapshot = valid_snapshot(1);
        let (health, sia) = m.compute_all(&snapshot).unwrap();
        assert!(health.nh > 0.0);
        assert!(sia.sia >= 1.0);
    }

    #[test]
    fn test_is_healthy() {
        let m = GlobalMetrics::new();
        assert!(m.is_healthy(0.8));
        assert!(!m.is_healthy(0.4));
    }

    #[test]
    fn test_needs_quarantine() {
        let m = GlobalMetrics::new();
        assert!(m.needs_quarantine(0.2));
        assert!(!m.needs_quarantine(0.5));
    }

    #[test]
    fn test_needs_apoptosis() {
        let m = GlobalMetrics::new();
        assert!(m.needs_apoptosis(0.05));
        assert!(!m.needs_apoptosis(0.2));
    }

    #[test]
    fn test_average_health() {
        let mut m = GlobalMetrics::new();
        for i in 1..=10 {
            m.compute_health(&valid_snapshot(i)).unwrap();
        }
        let avg = m.average_health(10).unwrap();
        assert!(avg > 0.0);
    }

    #[test]
    fn test_average_health_empty() {
        let m = GlobalMetrics::new();
        assert!(m.average_health(5).is_none());
    }

    #[test]
    fn test_health_trend_improving() {
        let mut m = GlobalMetrics::new();
        // Low health first
        m.compute_health(&NetworkSnapshot {
            mean_z_sct: 0.2,
            ..valid_snapshot(1)
        })
        .unwrap();
        // High health later
        m.compute_health(&NetworkSnapshot {
            mean_z_sct: 0.9,
            ..valid_snapshot(2)
        })
        .unwrap();
        let trend = m.health_trend(2).unwrap();
        assert!(trend > 0.0);
    }

    #[test]
    fn test_health_trend_declining() {
        let mut m = GlobalMetrics::new();
        m.compute_health(&NetworkSnapshot {
            mean_z_sct: 0.9,
            ..valid_snapshot(1)
        })
        .unwrap();
        m.compute_health(&NetworkSnapshot {
            mean_z_sct: 0.2,
            ..valid_snapshot(2)
        })
        .unwrap();
        let trend = m.health_trend(2).unwrap();
        assert!(trend < 0.0);
    }

    #[test]
    fn test_health_trend_insufficient() {
        let mut m = GlobalMetrics::new();
        m.compute_health(&valid_snapshot(1)).unwrap();
        assert!(m.health_trend(2).is_none());
    }

    #[test]
    fn test_reset() {
        let mut m = GlobalMetrics::new();
        m.compute_health(&valid_snapshot(1)).unwrap();
        m.compute_sia(&valid_snapshot(1)).unwrap();
        m.reset();
        assert!(m.latest_health().is_none());
        assert!(m.latest_sia().is_none());
    }

    #[test]
    fn test_update_config() {
        let mut m = GlobalMetrics::new();
        let config = MetricsConfig {
            weight_ethical: 0.5,
            weight_emergence: 0.25,
            weight_attractor: 0.25,
            ..Default::default()
        };
        m.update_config(config).unwrap();
        assert_eq!(m.config.weight_ethical, 0.5);
    }

    #[test]
    fn test_default() {
        let m = GlobalMetrics::default();
        assert!(m.latest_health().is_none());
    }

    #[test]
    fn test_error_display() {
        let err = MetricsError::InsufficientData("test".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("test"));
    }

    #[test]
    fn test_history_bounded() {
        let mut m = GlobalMetrics::new();
        for i in 1..=2000 {
            m.compute_health(&valid_snapshot(i)).unwrap();
        }
        assert!(m.history.len() <= 1000);
    }

    #[test]
    fn test_emergence_rate_tanh_clamping() {
        let mut m = GlobalMetrics::new();
        let snapshot = NetworkSnapshot {
            macro_concepts_born: 10000,
            ..valid_snapshot(1)
        };
        let score = m.compute_health(&snapshot).unwrap();
        assert!(score.emergence_rate <= 1.0);
    }

    #[test]
    fn test_config_validate_valid() {
        let config = MetricsConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_negative_weight_rejected() {
        let config = MetricsConfig {
            weight_ethical: -0.1,
            weight_emergence: 0.55,
            weight_attractor: 0.55,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }
}
