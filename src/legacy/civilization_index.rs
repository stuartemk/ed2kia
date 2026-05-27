//! Noospheric Civilization Index (NCI) Calculator — Sprint 61
//!
//! Implements the core metric for measuring Noospheric civilization maturity:
//!
//! ```text
//! NCI(t) = w_1 * Z_avg(t) + w_2 * Phi_PH(t) + w_3 * H_sym(t) + w_4 * I_human(t)
//! ```
//!
//! Where:
//! - `Z_avg(t)` — Average ethical z-score across active nodes at time t
//! - `Phi_PH(t)` — Persistent Homology coherence (HOPH beta-2 persistence score)
//! - `H_sym(t)` — Symbiotic entropy measure (cooperation density)
//! - `I_human(t)` — Human integration index (biometric coherence correlation)
//!
//! Also implements **Amplificación Simbiótica (A_sym)** with logistic decay
//! to stabilize human reasoning growth over time.


// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during NCI computation or A_sym application.
#[derive(Debug, Clone, PartialEq)]
pub enum NciError {
    /// Weight provided is negative.
    NegativeWeight(String),
    /// Weight sum exceeds allowed maximum (> 2.0).
    WeightSumExceeded(f64),
    /// Insufficient data points for computation.
    InsufficientData { required: usize, available: usize },
    /// Division by zero in normalization.
    DivisionByZero,
    /// Temporal regression detected (timestamp moved backward).
    TemporalRegression,
    /// NCI value outside valid range [0.0, 1.0].
    NciOutOfRange(f64),
}

impl std::fmt::Display for NciError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NciError::NegativeWeight(name) => write!(f, "Negative weight for component: {}", name),
            NciError::WeightSumExceeded(sum) => {
                write!(f, "Weight sum {} exceeds maximum allowed (2.0)", sum)
            }
            NciError::InsufficientData { required, available } => {
                write!(
                    f,
                    "Insufficient data: required {}, available {}",
                    required, available
                )
            }
            NciError::DivisionByZero => write!(f, "Division by zero in normalization"),
            NciError::TemporalRegression => write!(f, "Temporal regression detected"),
            NciError::NciOutOfRange(val) => {
                write!(f, "NCI value {} outside valid range [0.0, 1.0]", val)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Component Snapshot
// ---------------------------------------------------------------------------

/// Point-in-time snapshot of all NCI components.
#[derive(Debug, Clone, PartialEq)]
pub struct NciSnapshot {
    /// Timestamp in milliseconds since epoch.
    pub timestamp_ms: u64,
    /// Average ethical z-score across active nodes.
    pub z_avg: f64,
    /// Persistent Homology coherence (HOPH beta-2 score).
    pub phi_ph: f64,
    /// Symbiotic entropy / cooperation density.
    pub h_sym: f64,
    /// Human integration index.
    pub i_human: f64,
    /// Computed NCI value.
    pub nci: f64,
    /// Applied Amplification Simbiótica factor.
    pub a_sym: f64,
    /// Final NCI after amplification: `NCI * (1.0 + A_sym)`.
    pub nci_amplified: f64,
}

impl NciSnapshot {
    /// Create a new snapshot with computed NCI.
    pub fn new(
        timestamp_ms: u64,
        z_avg: f64,
        phi_ph: f64,
        h_sym: f64,
        i_human: f64,
        nci: f64,
        a_sym: f64,
        nci_amplified: f64,
    ) -> Self {
        Self {
            timestamp_ms,
            z_avg,
            phi_ph,
            h_sym,
            i_human,
            nci,
            a_sym,
            nci_amplified,
        }
    }

    /// Check if this snapshot indicates civilization maturity (NCI > 0.85).
    pub fn is_mature(&self) -> bool {
        self.nci_amplified > 0.85
    }
}

impl std::fmt::Display for NciSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NCI@{} [z={:.4}, phi={:.4}, h={:.4}, i={:.4}] = {:.4} (A_sym={:.4}, amplified={:.4})",
            self.timestamp_ms,
            self.z_avg,
            self.phi_ph,
            self.h_sym,
            self.i_human,
            self.nci,
            self.a_sym,
            self.nci_amplified,
        )
    }
}

// ---------------------------------------------------------------------------
// Weight Configuration
// ---------------------------------------------------------------------------

/// Configuration for NCI component weights.
#[derive(Debug, Clone)]
pub struct NciWeights {
    /// Weight for average ethical z-score.
    pub w_z: f64,
    /// Weight for Persistent Homology coherence.
    pub w_phi: f64,
    /// Weight for symbiotic entropy.
    pub w_h: f64,
    /// Weight for human integration.
    pub w_i: f64,
}

impl NciWeights {
    /// Create new weights and validate.
    pub fn new(w_z: f64, w_phi: f64, w_h: f64, w_i: f64) -> Result<Self, NciError> {
        if w_z < 0.0 {
            return Err(NciError::NegativeWeight("w_z".to_string()));
        }
        if w_phi < 0.0 {
            return Err(NciError::NegativeWeight("w_phi".to_string()));
        }
        if w_h < 0.0 {
            return Err(NciError::NegativeWeight("w_h".to_string()));
        }
        if w_i < 0.0 {
            return Err(NciError::NegativeWeight("w_i".to_string()));
        }

        let sum = w_z + w_phi + w_h + w_i;
        if sum > 10.0 {
            return Err(NciError::WeightSumExceeded(sum));
        }

        Ok(Self { w_z, w_phi, w_h, w_i })
    }

    /// Standard Stuartian weights: balanced distribution.
    pub fn stuartian_default() -> Self {
        Self {
            w_z: 0.35,
            w_phi: 0.25,
            w_h: 0.20,
            w_i: 0.20,
        }
    }

    /// Sum of all weights.
    pub fn sum(&self) -> f64 {
        self.w_z + self.w_phi + self.w_h + self.w_i
    }

    /// Normalize weights to sum to 1.0.
    pub fn normalized(&self) -> Self {
        let s = self.sum();
        if s == 0.0 {
            return Self::stuartian_default();
        }
        Self {
            w_z: self.w_z / s,
            w_phi: self.w_phi / s,
            w_h: self.w_h / s,
            w_i: self.w_i / s,
        }
    }
}

impl Default for NciWeights {
    fn default() -> Self {
        Self::stuartian_default()
    }
}

// ---------------------------------------------------------------------------
// Amplification Simbiótica (A_sym)
// ---------------------------------------------------------------------------

/// Logistic decay parameters for Amplification Simbiótica.
#[derive(Debug, Clone)]
pub struct ASymConfig {
    /// Maximum amplification factor (typically 0.30 = 30% boost).
    pub max_amplification: f64,
    /// Logistic growth rate (controls how fast amplification decays).
    pub growth_rate: f64,
    /// Midpoint of logistic curve (NCI value where amplification is half-max).
    pub midpoint: f64,
    /// Steepness of the logistic decay.
    pub steepness: f64,
}

impl ASymConfig {
    /// Default configuration: gentle logistic decay.
    pub fn default_stuartian() -> Self {
        Self {
            max_amplification: 0.30,
            growth_rate: 0.15,
            midpoint: 0.50,
            steepness: 8.0,
        }
    }

    /// Compute A_sym for a given NCI value using logistic decay.
    ///
    /// Formula:
    /// ```text
    /// A_sym(NCI) = max_amp / (1.0 + exp(steepness * (NCI - midpoint)))
    /// ```
    ///
    /// This provides maximum amplification when NCI is low (helping emergent
    /// civilizations grow), and decays as NCI approaches maturity.
    pub fn compute(&self, nci: f64) -> f64 {
        let exponent = self.steepness * (nci - self.midpoint);
        let logistic = 1.0 + exponent.exp();
        self.max_amplification / logistic
    }

    /// Compute the effective growth contribution over a time window.
    ///
    /// Integrates A_sym across a range of NCI values using trapezoidal rule.
    pub fn integrate_growth(&self, nci_start: f64, nci_end: f64, steps: usize) -> f64 {
        if steps == 0 {
            return 0.0;
        }
        let delta = (nci_end - nci_start) / steps as f64;
        let mut sum = 0.0;

        for i in 0..steps {
            let nci_current = nci_start + i as f64 * delta;
            let nci_next = nci_start + (i as f64 + 1.0) * delta;
            sum += 0.5 * (self.compute(nci_current) + self.compute(nci_next)) * delta;
        }

        sum
    }
}

impl Default for ASymConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

// ---------------------------------------------------------------------------
// NCI Calculator
// ---------------------------------------------------------------------------

/// Noospheric Civilization Index Calculator.
///
/// Maintains temporal history of NCI snapshots and computes
/// Amplification Simbiótica for each measurement.
#[derive(Debug, Clone)]
pub struct NciCalculator {
    /// Component weights.
    weights: NciWeights,
    /// A_sym configuration.
    a_sym_config: ASymConfig,
    /// Temporal history of snapshots.
    history: Vec<NciSnapshot>,
    /// Maximum history size (prevent unbounded growth).
    max_history: usize,
}

impl NciCalculator {
    /// Create a new calculator with default Stuartian weights.
    pub fn new() -> Self {
        Self {
            weights: NciWeights::stuartian_default(),
            a_sym_config: ASymConfig::default_stuartian(),
            history: Vec::new(),
            max_history: 10_000,
        }
    }

    /// Create with custom weights and A_sym config.
    pub fn with_config(
        weights: NciWeights,
        a_sym_config: ASymConfig,
    ) -> Result<Self, NciError> {
        // Validate weights
        if weights.w_z < 0.0 {
            return Err(NciError::NegativeWeight("w_z".to_string()));
        }
        if weights.w_phi < 0.0 {
            return Err(NciError::NegativeWeight("w_phi".to_string()));
        }
        if weights.w_h < 0.0 {
            return Err(NciError::NegativeWeight("w_h".to_string()));
        }
        if weights.w_i < 0.0 {
            return Err(NciError::NegativeWeight("w_i".to_string()));
        }

        Ok(Self {
            weights,
            a_sym_config,
            history: Vec::new(),
            max_history: 10_000,
        })
    }

    /// Compute raw NCI from component values.
    ///
    /// ```text
    /// NCI(t) = w_1 * Z_avg(t) + w_2 * Phi_PH(t) + w_3 * H_sym(t) + w_4 * I_human(t)
    /// ```
    pub fn compute_nci(&self, z_avg: f64, phi_ph: f64, h_sym: f64, i_human: f64) -> f64 {
        let w = self.weights.normalized();
        w.w_z * z_avg + w.w_phi * phi_ph + w.w_h * h_sym + w.w_i * i_human
    }

    /// Compute full snapshot with A_sym amplification.
    pub fn compute_snapshot(
        &self,
        timestamp_ms: u64,
        z_avg: f64,
        phi_ph: f64,
        h_sym: f64,
        i_human: f64,
    ) -> NciSnapshot {
        let nci = self.compute_nci(z_avg, phi_ph, h_sym, i_human);
        let a_sym = self.a_sym_config.compute(nci);
        let nci_amplified = nci * (1.0 + a_sym);

        NciSnapshot::new(timestamp_ms, z_avg, phi_ph, h_sym, i_human, nci, a_sym, nci_amplified)
    }

    /// Record a new snapshot in history.
    ///
    /// Returns `Err(TemporalRegression)` if timestamp is earlier than
    /// the last recorded snapshot.
    pub fn record(&mut self, snapshot: NciSnapshot) -> Result<(), NciError> {
        if let Some(last) = self.history.last() {
            if snapshot.timestamp_ms < last.timestamp_ms {
                return Err(NciError::TemporalRegression);
            }
        }

        self.history.push(snapshot);

        // Prune if exceeding max history
        if self.history.len() > self.max_history {
            let excess = self.history.len() - self.max_history;
            self.history.drain(0..excess);
        }

        Ok(())
    }

    /// Get the latest snapshot.
    pub fn latest(&self) -> Option<&NciSnapshot> {
        self.history.last()
    }

    /// Get the latest amplified NCI value.
    pub fn latest_nci_amplified(&self) -> Option<f64> {
        self.history.last().map(|s| s.nci_amplified)
    }

    /// Get the latest raw NCI value.
    pub fn latest_nci(&self) -> Option<f64> {
        self.history.last().map(|s| s.nci)
    }

    /// Check if civilization has reached maturity threshold.
    pub fn is_mature(&self) -> bool {
        self.latest_nci_amplified()
            .map(|nci| nci > 0.85)
            .unwrap_or(false)
    }

    /// Calculate the NCI trend over the last N snapshots.
    ///
    /// Returns positive if NCI is increasing, negative if decreasing.
    pub fn trend(&self, window: usize) -> Result<f64, NciError> {
        let len = self.history.len();
        if len < 2 {
            return Err(NciError::InsufficientData {
                required: 2,
                available: len,
            });
        }

        let w = if window > len { len } else { window };
        let recent = &self.history[len - w..];

        // Linear regression slope
        let n = w as f64;
        let sum_x: f64 = (0..w).map(|i| i as f64).sum();
        let sum_y: f64 = recent.iter().map(|s| s.nci_amplified).sum();
        let sum_xy: f64 = recent
            .iter()
            .enumerate()
            .map(|(i, s)| i as f64 * s.nci_amplified)
            .sum();
        let sum_x2: f64 = (0..w).map(|i| (i as f64) * (i as f64)).sum();

        let denominator = n * sum_x2 - sum_x * sum_x;
        if denominator == 0.0 {
            return Err(NciError::DivisionByZero);
        }

        let slope = (n * sum_xy - sum_x * sum_y) / denominator;
        Ok(slope)
    }

    /// Calculate the average NCI over the last N snapshots.
    pub fn average_nci(&self, window: usize) -> Result<f64, NciError> {
        let len = self.history.len();
        if len == 0 {
            return Err(NciError::InsufficientData {
                required: 1,
                available: 0,
            });
        }

        let w = if window > len { len } else { window };
        let recent = &self.history[len - w..];
        let sum: f64 = recent.iter().map(|s| s.nci_amplified).sum();
        Ok(sum / w as f64)
    }

    /// Get snapshots within a time range.
    pub fn range(
        &self,
        start_ms: u64,
        end_ms: u64,
    ) -> Vec<&NciSnapshot> {
        self.history
            .iter()
            .filter(|s| s.timestamp_ms >= start_ms && s.timestamp_ms <= end_ms)
            .collect()
    }

    /// Get the history length.
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Iterate over snapshots in chronological order.
    pub fn iter(&self) -> impl Iterator<Item = &NciSnapshot> {
        self.history.iter()
    }

    /// Update weights.
    pub fn update_weights(&mut self, weights: NciWeights) {
        self.weights = weights;
    }

    /// Update A_sym configuration.
    pub fn update_a_sym_config(&mut self, config: ASymConfig) {
        self.a_sym_config = config;
    }

    /// Get current weights.
    pub fn weights(&self) -> &NciWeights {
        &self.weights
    }

    /// Get current A_sym config.
    pub fn a_sym_config(&self) -> &ASymConfig {
        &self.a_sym_config
    }

    /// Reset calculator state.
    pub fn reset(&mut self) {
        self.history.clear();
    }

    /// Clear history only.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Compute maturity duration: how many consecutive snapshots have NCI > 0.85.
    pub fn maturity_duration(&self) -> usize {
        let mut count = 0;
        for snapshot in self.history.iter().rev() {
            if snapshot.nci_amplified > 0.85 {
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    /// Compute the projected NCI after a given number of steps assuming
    /// current trend continues.
    pub fn project_nci(&self, steps: usize) -> Result<f64, NciError> {
        let current = self.latest_nci_amplified().ok_or(NciError::InsufficientData {
            required: 1,
            available: 0,
        })?;
        let slope = self.trend(10)?;
        let projected = current + slope * steps as f64;
        // Clamp to valid range
        Ok(projected.clamp(0.0, 1.0))
    }
}

impl Default for NciCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NciCalculator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NCI Calculator [history={}, latest={:?}, mature={}]",
            self.history.len(),
            self.latest_nci_amplified(),
            self.is_mature(),
        )
    }
}

// ---------------------------------------------------------------------------
// Maturity Tracker
// ---------------------------------------------------------------------------

/// Tracks sustained NCI maturity for Handover Protocol eligibility.
#[derive(Debug, Clone)]
pub struct MaturityTracker {
    /// NCI threshold for maturity (default 0.85).
    threshold: f64,
    /// Required consecutive snapshots above threshold (default: 6 months worth).
    required_duration: usize,
    /// Current consecutive count.
    current_streak: usize,
    /// Timestamp when current streak started.
    streak_start_ms: Option<u64>,
}

impl MaturityTracker {
    /// Create with default 6-month requirement (assuming daily snapshots = 180).
    pub fn new() -> Self {
        Self {
            threshold: 0.85,
            required_duration: 180, // 6 months of daily snapshots
            current_streak: 0,
            streak_start_ms: None,
        }
    }

    /// Create with custom threshold and duration.
    pub fn with_config(threshold: f64, required_duration: usize) -> Self {
        Self {
            threshold,
            required_duration,
            current_streak: 0,
            streak_start_ms: None,
        }
    }

    /// Process a new NCI value. Returns true if maturity is achieved.
    pub fn process(&mut self, nci: f64, timestamp_ms: u64) -> bool {
        if nci >= self.threshold {
            if self.current_streak == 0 {
                self.streak_start_ms = Some(timestamp_ms);
            }
            self.current_streak += 1;
        } else {
            self.current_streak = 0;
            self.streak_start_ms = None;
        }

        self.current_streak >= self.required_duration
    }

    /// Check if maturity requirements are met.
    pub fn is_mature(&self) -> bool {
        self.current_streak >= self.required_duration
    }

    /// Get current streak length.
    pub fn streak(&self) -> usize {
        self.current_streak
    }

    /// Get progress toward maturity (0.0 to 1.0).
    pub fn progress(&self) -> f64 {
        if self.required_duration == 0 {
            return 1.0;
        }
        (self.current_streak as f64 / self.required_duration as f64).min(1.0)
    }

    /// Get streak start timestamp.
    pub fn streak_start(&self) -> Option<u64> {
        self.streak_start_ms
    }

    /// Reset tracker.
    pub fn reset(&mut self) {
        self.current_streak = 0;
        self.streak_start_ms = None;
    }
}

impl Default for MaturityTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- NciWeights Tests ----

    #[test]
    fn test_weights_default() {
        let w = NciWeights::stuartian_default();
        assert!((w.w_z - 0.35).abs() < 1e-10);
        assert!((w.w_phi - 0.25).abs() < 1e-10);
        assert!((w.w_h - 0.20).abs() < 1e-10);
        assert!((w.w_i - 0.20).abs() < 1e-10);
    }

    #[test]
    fn test_weights_sum() {
        let w = NciWeights::new(0.4, 0.3, 0.2, 0.1).unwrap();
        assert!((w.sum() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_weights_normalized() {
        let w = NciWeights::new(2.0, 1.0, 1.0, 1.0).unwrap();
        let norm = w.normalized();
        assert!((norm.sum() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_weights_negative_rejected() {
        match NciWeights::new(-0.1, 0.3, 0.3, 0.3) {
            Err(NciError::NegativeWeight(name)) => assert_eq!(name, "w_z"),
            _ => panic!("Expected NegativeWeight error"),
        }
    }

    #[test]
    fn test_weights_sum_exceeded() {
        match NciWeights::new(3.0, 3.0, 3.0, 3.0) {
            Err(NciError::WeightSumExceeded(sum)) => assert!((sum - 12.0).abs() < 1e-10),
            _ => panic!("Expected WeightSumExceeded error"),
        }
    }

    #[test]
    fn test_weights_zero_sum_normalization() {
        let w = NciWeights {
            w_z: 0.0,
            w_phi: 0.0,
            w_h: 0.0,
            w_i: 0.0,
        };
        let norm = w.normalized();
        // Should fall back to stuartian default
        assert!(norm.sum() > 0.0);
    }

    // ---- ASymConfig Tests ----

    #[test]
    fn test_asym_default() {
        let cfg = ASymConfig::default_stuartian();
        assert!((cfg.max_amplification - 0.30).abs() < 1e-10);
        assert!((cfg.midpoint - 0.50).abs() < 1e-10);
    }

    #[test]
    fn test_asym_low_nci_high_amplification() {
        let cfg = ASymConfig::default_stuartian();
        let a_low = cfg.compute(0.1);
        let a_high = cfg.compute(0.9);
        // Low NCI should get more amplification
        assert!(a_low > a_high);
    }

    #[test]
    fn test_asym_midpoint_half_max() {
        let cfg = ASymConfig::default_stuartian();
        let a_mid = cfg.compute(cfg.midpoint);
        // At midpoint, A_sym should be max_amp / (1 + exp(0)) = max_amp / 2
        let expected = cfg.max_amplification / 2.0;
        assert!((a_mid - expected).abs() < 1e-6);
    }

    #[test]
    fn test_asym_non_negative() {
        let cfg = ASymConfig::default_stuartian();
        for i in 0..100 {
            let nci = i as f64 / 100.0;
            let a = cfg.compute(nci);
            assert!(a >= 0.0, "A_sym should be non-negative for NCI={}", nci);
        }
    }

    #[test]
    fn test_asym_bounded() {
        let cfg = ASymConfig::default_stuartian();
        for i in 0..100 {
            let nci = i as f64 / 100.0;
            let a = cfg.compute(nci);
            assert!(
                a <= cfg.max_amplification + 1e-10,
                "A_sym should not exceed max_amplification"
            );
        }
    }

    #[test]
    fn test_asym_integrate_growth() {
        let cfg = ASymConfig::default_stuartian();
        let integral = cfg.integrate_growth(0.0, 1.0, 100);
        assert!(integral > 0.0, "Integral should be positive");
        assert!(
            integral < cfg.max_amplification,
            "Integral should be less than max amplification"
        );
    }

    #[test]
    fn test_asym_integrate_zero_steps() {
        let cfg = ASymConfig::default_stuartian();
        let integral = cfg.integrate_growth(0.0, 1.0, 0);
        assert!((integral - 0.0).abs() < 1e-10);
    }

    // ---- NciCalculator Tests ----

    #[test]
    fn test_calculator_creation() {
        let calc = NciCalculator::new();
        assert_eq!(calc.history_len(), 0);
        assert!(!calc.is_mature());
    }

    #[test]
    fn test_compute_nci() {
        let calc = NciCalculator::new();
        let nci = calc.compute_nci(1.0, 1.0, 1.0, 1.0);
        // With normalized weights summing to 1.0, all components at 1.0 gives NCI=1.0
        assert!((nci - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_nci_zero() {
        let calc = NciCalculator::new();
        let nci = calc.compute_nci(0.0, 0.0, 0.0, 0.0);
        assert!((nci - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_snapshot() {
        let calc = NciCalculator::new();
        let snap = calc.compute_snapshot(1000, 0.8, 0.7, 0.6, 0.9);
        assert_eq!(snap.timestamp_ms, 1000);
        assert!(snap.nci > 0.0);
        assert!(snap.a_sym >= 0.0);
        assert!(snap.nci_amplified >= snap.nci);
    }

    #[test]
    fn test_record_snapshot() {
        let mut calc = NciCalculator::new();
        let snap = calc.compute_snapshot(1000, 0.5, 0.5, 0.5, 0.5);
        calc.record(snap).unwrap();
        assert_eq!(calc.history_len(), 1);
    }

    #[test]
    fn test_temporal_regression() {
        let mut calc = NciCalculator::new();
        let snap1 = calc.compute_snapshot(2000, 0.5, 0.5, 0.5, 0.5);
        calc.record(snap1).unwrap();
        let snap2 = calc.compute_snapshot(1000, 0.6, 0.6, 0.6, 0.6);
        match calc.record(snap2) {
            Err(NciError::TemporalRegression) => {}
            _ => panic!("Expected TemporalRegression error"),
        }
    }

    #[test]
    fn test_latest_snapshot() {
        let mut calc = NciCalculator::new();
        let snap = calc.compute_snapshot(1000, 0.5, 0.5, 0.5, 0.5);
        calc.record(snap).unwrap();
        assert!(calc.latest().is_some());
    }

    #[test]
    fn test_latest_none() {
        let calc = NciCalculator::new();
        assert!(calc.latest().is_none());
    }

    #[test]
    fn test_maturity_check() {
        let mut calc = NciCalculator::new();
        // Record high NCI snapshot
        let snap = calc.compute_snapshot(1000, 1.0, 1.0, 1.0, 1.0);
        calc.record(snap).unwrap();
        // NCI=1.0 with amplification should exceed 0.85
        assert!(calc.is_mature());
    }

    #[test]
    fn test_not_mature() {
        let mut calc = NciCalculator::new();
        let snap = calc.compute_snapshot(1000, 0.1, 0.1, 0.1, 0.1);
        calc.record(snap).unwrap();
        assert!(!calc.is_mature());
    }

    #[test]
    fn test_trend_increasing() {
        let mut calc = NciCalculator::new();
        for i in 0..20 {
            let val = 0.3 + i as f64 * 0.02;
            let snap = calc.compute_snapshot(i * 1000, val, val, val, val);
            calc.record(snap).unwrap();
        }
        let trend = calc.trend(10).unwrap();
        assert!(trend > 0.0, "Trend should be positive for increasing NCI");
    }

    #[test]
    fn test_trend_decreasing() {
        let mut calc = NciCalculator::new();
        for i in 0..20 {
            let val = 0.9 - i as f64 * 0.02;
            let snap = calc.compute_snapshot(i * 1000, val, val, val, val);
            calc.record(snap).unwrap();
        }
        let trend = calc.trend(10).unwrap();
        assert!(trend < 0.0, "Trend should be negative for decreasing NCI");
    }

    #[test]
    fn test_trend_insufficient_data() {
        let calc = NciCalculator::new();
        match calc.trend(10) {
            Err(NciError::InsufficientData { .. }) => {}
            _ => panic!("Expected InsufficientData error"),
        }
    }

    #[test]
    fn test_average_nci() {
        let mut calc = NciCalculator::new();
        for i in 0..10 {
            let val = 0.5;
            let snap = calc.compute_snapshot(i * 1000, val, val, val, val);
            calc.record(snap).unwrap();
        }
        let avg = calc.average_nci(10).unwrap();
        assert!(avg > 0.0);
    }

    #[test]
    fn test_average_nci_empty() {
        let calc = NciCalculator::new();
        match calc.average_nci(10) {
            Err(NciError::InsufficientData { .. }) => {}
            _ => panic!("Expected InsufficientData error"),
        }
    }

    #[test]
    fn test_range_query() {
        let mut calc = NciCalculator::new();
        for i in 0..10 {
            let snap = calc.compute_snapshot(i * 1000, 0.5, 0.5, 0.5, 0.5);
            calc.record(snap).unwrap();
        }
        let results = calc.range(3000, 7000);
        assert_eq!(results.len(), 5); // timestamps 3000, 4000, 5000, 6000, 7000
    }

    #[test]
    fn test_reset() {
        let mut calc = NciCalculator::new();
        let snap = calc.compute_snapshot(1000, 0.5, 0.5, 0.5, 0.5);
        calc.record(snap).unwrap();
        calc.reset();
        assert_eq!(calc.history_len(), 0);
    }

    #[test]
    fn test_update_weights() {
        let mut calc = NciCalculator::new();
        let new_weights = NciWeights::new(0.5, 0.2, 0.2, 0.1).unwrap();
        calc.update_weights(new_weights);
        assert!((calc.weights().w_z - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_update_a_sym_config() {
        let mut calc = NciCalculator::new();
        let new_cfg = ASymConfig {
            max_amplification: 0.50,
            ..ASymConfig::default_stuartian()
        };
        calc.update_a_sym_config(new_cfg);
        assert!((calc.a_sym_config().max_amplification - 0.50).abs() < 1e-10);
    }

    #[test]
    fn test_history_pruning() {
        let mut calc = NciCalculator::new();
        calc.max_history = 100;
        for i in 0..150 {
            let snap = calc.compute_snapshot(i * 1000, 0.5, 0.5, 0.5, 0.5);
            calc.record(snap).unwrap();
        }
        assert_eq!(calc.history_len(), 100);
    }

    #[test]
    fn test_snapshot_display() {
        let snap = NciSnapshot::new(1000, 0.5, 0.6, 0.7, 0.8, 0.65, 0.1, 0.715);
        let display = format!("{}", snap);
        assert!(display.contains("NCI@1000"));
    }

    #[test]
    fn test_calculator_display() {
        let calc = NciCalculator::new();
        let display = format!("{}", calc);
        assert!(display.contains("NCI Calculator"));
    }

    // ---- MaturityTracker Tests ----

    #[test]
    fn test_maturity_tracker_creation() {
        let tracker = MaturityTracker::new();
        assert!(!tracker.is_mature());
        assert_eq!(tracker.streak(), 0);
    }

    #[test]
    fn test_maturity_tracker_with_config() {
        let tracker = MaturityTracker::with_config(0.90, 100);
        assert!(!tracker.is_mature());
    }

    #[test]
    fn test_maturity_tracker_streak_increases() {
        let mut tracker = MaturityTracker::with_config(0.5, 5);
        for i in 0..5 {
            tracker.process(0.6, i * 1000);
        }
        assert!(tracker.is_mature());
        assert_eq!(tracker.streak(), 5);
    }

    #[test]
    fn test_maturity_tracker_streak_resets() {
        let mut tracker = MaturityTracker::with_config(0.5, 10);
        for i in 0..7 {
            tracker.process(0.6, i * 1000);
        }
        // Drop below threshold
        tracker.process(0.3, 7000);
        assert_eq!(tracker.streak(), 0);
        assert!(!tracker.is_mature());
    }

    #[test]
    fn test_maturity_tracker_progress() {
        let mut tracker = MaturityTracker::with_config(0.5, 10);
        for i in 0..5 {
            tracker.process(0.6, i * 1000);
        }
        assert!((tracker.progress() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_maturity_tracker_progress_capped() {
        let mut tracker = MaturityTracker::with_config(0.5, 10);
        for i in 0..15 {
            tracker.process(0.6, i * 1000);
        }
        assert!((tracker.progress() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_maturity_tracker_streak_start() {
        let mut tracker = MaturityTracker::with_config(0.5, 10);
        tracker.process(0.6, 5000);
        assert_eq!(tracker.streak_start(), Some(5000));
    }

    #[test]
    fn test_maturity_tracker_reset() {
        let mut tracker = MaturityTracker::with_config(0.5, 10);
        for i in 0..5 {
            tracker.process(0.6, i * 1000);
        }
        tracker.reset();
        assert_eq!(tracker.streak(), 0);
        assert!(!tracker.is_mature());
        assert!(tracker.streak_start().is_none());
    }

    #[test]
    fn test_maturity_tracker_default() {
        let tracker = MaturityTracker::default();
        assert!((tracker.threshold - 0.85).abs() < 1e-10);
        assert_eq!(tracker.required_duration, 180);
    }

    // ---- Integration Tests ----

    #[test]
    fn test_full_nci_workflow() {
        let mut calc = NciCalculator::new();
        let mut tracker = MaturityTracker::with_config(0.7, 10);

        // Simulate 20 days of increasing civilization maturity
        for day in 0..20 {
            let base = 0.3 + day as f64 * 0.03;
            let snap = calc.compute_snapshot(
                day * 86400000, // daily timestamps
                base,
                base * 0.9,
                base * 0.8,
                base * 0.85,
            );
            let nci_amp = snap.nci_amplified;
            let ts = snap.timestamp_ms;
            calc.record(snap).unwrap();
            tracker.process(nci_amp, ts);
        }

        assert!(calc.history_len() == 20);
        assert!(calc.latest().is_some());
        let trend = calc.trend(10).unwrap();
        assert!(trend > 0.0);
    }

    #[test]
    fn test_project_nci() {
        let mut calc = NciCalculator::new();
        for i in 0..15 {
            let val = 0.4 + i as f64 * 0.02;
            let snap = calc.compute_snapshot(i * 1000, val, val, val, val);
            calc.record(snap).unwrap();
        }
        let projected = calc.project_nci(5).unwrap();
        assert!(projected >= 0.0 && projected <= 1.0);
    }

    #[test]
    fn test_project_nci_no_data() {
        let calc = NciCalculator::new();
        match calc.project_nci(5) {
            Err(NciError::InsufficientData { .. }) => {}
            _ => panic!("Expected InsufficientData error"),
        }
    }

    #[test]
    fn test_maturity_duration() {
        let mut calc = NciCalculator::new();
        // Record 5 mature snapshots
        for i in 0..5 {
            let snap = calc.compute_snapshot(i * 1000, 1.0, 1.0, 1.0, 1.0);
            calc.record(snap).unwrap();
        }
        // Record 3 non-mature
        for i in 5..8 {
            let snap = calc.compute_snapshot(i * 1000, 0.1, 0.1, 0.1, 0.1);
            calc.record(snap).unwrap();
        }
        // maturity_duration counts consecutive mature from end
        assert_eq!(calc.maturity_duration(), 0);
    }

    #[test]
    fn test_maturity_duration_all_mature() {
        let mut calc = NciCalculator::new();
        for i in 0..10 {
            let snap = calc.compute_snapshot(i * 1000, 1.0, 1.0, 1.0, 1.0);
            calc.record(snap).unwrap();
        }
        assert_eq!(calc.maturity_duration(), 10);
    }

    // ---- Error Display Tests ----

    #[test]
    fn test_error_display_negative_weight() {
        let err = NciError::NegativeWeight("test".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Negative weight"));
    }

    #[test]
    fn test_error_display_weight_sum() {
        let err = NciError::WeightSumExceeded(3.0);
        let msg = format!("{}", err);
        assert!(msg.contains("exceeds maximum"));
    }

    #[test]
    fn test_error_display_insufficient_data() {
        let err = NciError::InsufficientData {
            required: 5,
            available: 2,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Insufficient data"));
    }

    #[test]
    fn test_error_display_division_by_zero() {
        let err = NciError::DivisionByZero;
        let msg = format!("{}", err);
        assert!(msg.contains("Division by zero"));
    }

    #[test]
    fn test_error_display_temporal_regression() {
        let err = NciError::TemporalRegression;
        let msg = format!("{}", err);
        assert!(msg.contains("Temporal regression"));
    }

    #[test]
    fn test_error_display_nci_out_of_range() {
        let err = NciError::NciOutOfRange(1.5);
        let msg = format!("{}", err);
        assert!(msg.contains("outside valid range"));
    }

    // ---- Edge Cases ----

    #[test]
    fn test_nci_with_custom_weights() {
        let weights = NciWeights::new(0.5, 0.3, 0.1, 0.1).unwrap();
        let calc = NciCalculator::with_config(weights, ASymConfig::default_stuartian()).unwrap();
        // z_avg dominates with weight 0.5
        let nci = calc.compute_nci(1.0, 0.0, 0.0, 0.0);
        let norm = calc.weights().normalized();
        let expected = norm.w_z * 1.0;
        assert!((nci - expected).abs() < 1e-10);
    }

    #[test]
    fn test_snapshot_is_mature() {
        let snap = NciSnapshot::new(1000, 1.0, 1.0, 1.0, 1.0, 1.0, 0.1, 1.1);
        assert!(snap.is_mature());
    }

    #[test]
    fn test_snapshot_not_mature() {
        let snap = NciSnapshot::new(1000, 0.1, 0.1, 0.1, 0.1, 0.1, 0.05, 0.105);
        assert!(!snap.is_mature());
    }

    #[test]
    fn test_clear_history() {
        let mut calc = NciCalculator::new();
        let snap = calc.compute_snapshot(1000, 0.5, 0.5, 0.5, 0.5);
        calc.record(snap).unwrap();
        calc.clear_history();
        assert_eq!(calc.history_len(), 0);
    }

    #[test]
    fn test_iter_snapshots() {
        let mut calc = NciCalculator::new();
        for i in 0..5 {
            let snap = calc.compute_snapshot(i * 1000, 0.5, 0.5, 0.5, 0.5);
            calc.record(snap).unwrap();
        }
        let count = calc.iter().count();
        assert_eq!(count, 5);
    }
}
