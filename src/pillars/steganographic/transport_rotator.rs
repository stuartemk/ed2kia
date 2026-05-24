//! Transport Rotator — Dynamic Protocol Rotation for Network Preservation.
//!
//! Manages dynamic rotation between transport protocols (TCP, QUIC, WebSocket, WebRTC)
//! based on network health metrics, ensuring resilient cooperative communication
//! through adaptive protocol distribution.
//!
//! **Design Principles:**
//! - Health-based rotation: switches protocols based on latency, packet loss, and throughput.
//! - Session continuity: notifies orchestrator of endpoint changes without breaking active sessions.
//! - WASM-compatible control layer (no std::fs, no std::net).
//!
//! **Reference:** RFC 003 (Steganographic Survival), Sprint 45

use std::time::Duration;

/// Supported transport types for dynamic rotation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TransportType {
    /// Standard TCP transport — baseline reliability.
    Tcp,
    /// QUIC (HTTP/3) — low-latency, multiplexed streams.
    Quic,
    /// WebSocket — browser-compatible, firewall-friendly.
    WebSocket,
    /// WebRTC — P2P, NAT traversal, media-optimized.
    WebRtc,
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportType::Tcp => write!(f, "tcp"),
            TransportType::Quic => write!(f, "quic"),
            TransportType::WebSocket => write!(f, "websocket"),
            TransportType::WebRtc => write!(f, "webrtc"),
        }
    }
}

/// Errors specific to transport rotation.
#[derive(Debug, Clone, PartialEq)]
pub enum RotationError {
    /// No healthy transport available for rotation.
    NoHealthyTransport,
    /// Rotation interval too short (minimum 10s).
    IntervalTooShort(Duration),
    /// Empty protocol list — at least one transport required.
    EmptyProtocolList,
    /// Transport not in active protocol list.
    TransportNotAvailable(TransportType),
}

impl std::fmt::Display for RotationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RotationError::NoHealthyTransport => write!(f, "No healthy transport available for rotation"),
            RotationError::IntervalTooShort(d) => write!(f, "Rotation interval too short: {:?} (min 10s)", d),
            RotationError::EmptyProtocolList => write!(f, "Empty protocol list — at least one transport required"),
            RotationError::TransportNotAvailable(t) => write!(f, "Transport {} not in active protocol list", t),
        }
    }
}

/// Health metrics for a transport protocol.
#[derive(Debug, Clone)]
pub struct TransportHealth {
    /// Current transport type.
    pub transport: TransportType,
    /// Measured latency in milliseconds.
    pub latency_ms: f64,
    /// Packet loss ratio (0.0 = no loss, 1.0 = total loss).
    pub packet_loss: f64,
    /// Throughput in bytes per second.
    pub throughput_bps: f64,
    /// Whether this transport is considered healthy.
    pub is_healthy: bool,
    /// Last health check timestamp (milliseconds).
    pub last_check_ms: u64,
}

impl TransportHealth {
    /// Create a new health report.
    pub fn new(transport: TransportType, latency_ms: f64, packet_loss: f64, throughput_bps: f64) -> Self {
        Self {
            transport,
            latency_ms,
            packet_loss,
            throughput_bps,
            is_healthy: packet_loss < 0.1 && latency_ms < 500.0,
            last_check_ms: 0,
        }
    }

    /// Compute a health score (0.0 = worst, 1.0 = best).
    pub fn score(&self) -> f64 {
        let latency_score = if self.latency_ms < 100.0 {
            1.0
        } else if self.latency_ms > 500.0 {
            0.0
        } else {
            1.0 - (self.latency_ms - 100.0) / 400.0
        };
        let loss_score = 1.0 - self.packet_loss;
        let throughput_score = if self.throughput_bps > 1_000_000.0 {
            1.0
        } else {
            self.throughput_bps / 1_000_000.0
        };
        (latency_score * 0.4 + loss_score * 0.4 + throughput_score * 0.2).clamp(0.0, 1.0)
    }
}

/// Configuration for the TransportRotator.
#[derive(Debug, Clone)]
pub struct RotatorConfig {
    /// Active protocols available for rotation.
    pub active_protocols: Vec<TransportType>,
    /// Rotation interval (minimum 10 seconds).
    pub rotation_interval: Duration,
    /// Health threshold for considering a transport healthy.
    pub health_threshold: f64,
    /// Jitter range for rotation timing (prevents synchronized rotation).
    pub jitter_ms: u64,
}

impl Default for RotatorConfig {
    fn default() -> Self {
        Self {
            active_protocols: vec![
                TransportType::Tcp,
                TransportType::Quic,
                TransportType::WebSocket,
                TransportType::WebRtc,
            ],
            rotation_interval: Duration::from_secs(300),
            health_threshold: 0.5,
            jitter_ms: 5000,
        }
    }
}

/// Transport Rotator — Dynamic Protocol Selection.
///
/// Monitors transport health and rotates protocols to maintain
/// resilient cooperative communication across diverse network conditions.
pub struct TransportRotator {
    config: RotatorConfig,
    /// Current active transport.
    current_transport: TransportType,
    /// Health reports for each transport.
    health_reports: Vec<TransportHealth>,
    /// Rotation counter.
    rotation_count: usize,
    /// PRNG state for jitter.
    rng_state: u64,
}

impl TransportRotator {
    /// Create a new TransportRotator with default configuration.
    pub fn new() -> Self {
        let config = RotatorConfig::default();
        let current = config.active_protocols[0].clone();
        Self {
            config,
            current_transport: current,
            health_reports: Vec::new(),
            rotation_count: 0,
            rng_state: 0x1234_5678_ABCD_EF00,
        }
    }

    /// Create a TransportRotator with custom configuration.
    pub fn with_config(config: RotatorConfig) -> Result<Self, RotationError> {
        if config.active_protocols.is_empty() {
            return Err(RotationError::EmptyProtocolList);
        }
        if config.rotation_interval < Duration::from_secs(10) {
            return Err(RotationError::IntervalTooShort(config.rotation_interval));
        }
        let current = config.active_protocols[0].clone();
        Ok(Self {
            config,
            current_transport: current,
            health_reports: Vec::new(),
            rotation_count: 0,
            rng_state: 0x1234_5678_ABCD_EF00,
        })
    }

    /// Next PRNG value for jitter.
    fn next_rng(&mut self) -> u64 {
        self.rng_state = self.rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.rng_state
    }

    /// Update health report for a transport.
    pub fn update_health(&mut self, health: TransportHealth) {
        if let Some(existing) = self.health_reports.iter_mut().find(|h| h.transport == health.transport) {
            *existing = health;
        } else {
            self.health_reports.push(health);
        }
    }

    /// Update health for all transports with simulated metrics.
    pub fn update_health_batch(&mut self, reports: Vec<TransportHealth>) {
        for report in reports {
            self.update_health(report);
        }
    }

    /// Select the best healthy transport based on health scores.
    pub fn select_best(&self) -> Option<TransportType> {
        self.health_reports.iter()
            .filter(|h| h.is_healthy)
            .max_by(|a, b| a.score().partial_cmp(&b.score()).unwrap_or(std::cmp::Ordering::Equal))
            .map(|h| h.transport.clone())
    }

    /// Rotate to the next best transport.
    ///
    /// Returns the new transport type, or an error if no healthy transport is available.
    pub fn rotate(&mut self) -> Result<TransportType, RotationError> {
        // Find healthy transports from active list
        let healthy: Vec<&TransportHealth> = self.health_reports.iter()
            .filter(|h| {
                h.is_healthy &&
                self.config.active_protocols.contains(&h.transport) &&
                h.transport != self.current_transport
            })
            .collect();

        if healthy.is_empty() {
            // Fallback: cycle to next protocol in active list
            let current_idx = self.config.active_protocols.iter()
                .position(|t| t == &self.current_transport)
                .unwrap_or(0);
            let next_idx = (current_idx + 1) % self.config.active_protocols.len();
            let next = self.config.active_protocols[next_idx].clone();

            self.current_transport = next.clone();
            self.rotation_count += 1;
            return Ok(next);
        }

        // Select best healthy transport (different from current)
        let best = healthy.iter()
            .max_by(|a, b| (**a).score().partial_cmp(&(**b).score()).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        self.current_transport = best.transport.clone();
        self.rotation_count += 1;
        Ok(best.transport.clone())
    }

    /// Force rotation to a specific transport.
    pub fn force_transport(&mut self, transport: &TransportType) -> Result<(), RotationError> {
        if !self.config.active_protocols.contains(transport) {
            return Err(RotationError::TransportNotAvailable(transport.clone()));
        }
        self.current_transport = transport.clone();
        self.rotation_count += 1;
        Ok(())
    }

    /// Get the current active transport.
    pub fn current_transport(&self) -> &TransportType {
        &self.current_transport
    }

    /// Get health report for a specific transport.
    pub fn get_health(&self, transport: &TransportType) -> Option<&TransportHealth> {
        self.health_reports.iter().find(|h| &h.transport == transport)
    }

    /// Get all health reports.
    pub fn all_health_reports(&self) -> &[TransportHealth] {
        &self.health_reports
    }

    /// Get rotation count.
    pub fn rotation_count(&self) -> usize {
        self.rotation_count
    }

    /// Get the rotation interval with jitter.
    pub fn rotation_interval_with_jitter(&mut self) -> Duration {
        let jitter = self.next_rng() % self.config.jitter_ms;
        self.config.rotation_interval + Duration::from_millis(jitter)
    }

    /// Get the current configuration.
    pub fn config(&self) -> &RotatorConfig {
        &self.config
    }
}

impl Default for TransportRotator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_health(transport: TransportType, latency: f64, loss: f64, throughput: f64) -> TransportHealth {
        TransportHealth::new(transport, latency, loss, throughput)
    }

    #[test]
    fn test_rotator_creation() {
        let rotator = TransportRotator::new();
        assert_eq!(*rotator.current_transport(), TransportType::Tcp);
        assert_eq!(rotator.rotation_count(), 0);
    }

    #[test]
    fn test_rotator_custom_config() {
        let config = RotatorConfig {
            active_protocols: vec![TransportType::Quic, TransportType::WebSocket],
            rotation_interval: Duration::from_secs(60),
            health_threshold: 0.7,
            jitter_ms: 2000,
        };
        let rotator = TransportRotator::with_config(config).unwrap();
        assert_eq!(*rotator.current_transport(), TransportType::Quic);
    }

    #[test]
    fn test_empty_protocol_list_rejected() {
        let config = RotatorConfig {
            active_protocols: vec![],
            ..RotatorConfig::default()
        };
        match TransportRotator::with_config(config) {
            Err(RotationError::EmptyProtocolList) => {},
            other => panic!("Expected EmptyProtocolList, got {:?}", other),
        }
    }

    #[test]
    fn test_interval_too_short_rejected() {
        let config = RotatorConfig {
            rotation_interval: Duration::from_secs(5),
            ..RotatorConfig::default()
        };
        match TransportRotator::with_config(config) {
            Err(RotationError::IntervalTooShort(_)) => {},
            other => panic!("Expected IntervalTooShort, got {:?}", other),
        }
    }

    #[test]
    fn test_health_score_calculation() {
        let health = make_health(TransportType::Tcp, 50.0, 0.0, 1_000_000.0);
        assert_eq!(health.score(), 1.0);
        assert!(health.is_healthy);

        let bad = make_health(TransportType::Tcp, 600.0, 0.5, 100.0);
        assert!(!bad.is_healthy);
        assert!(bad.score() < 0.5);
    }

    #[test]
    fn test_update_health() {
        let mut rotator = TransportRotator::new();
        let health = make_health(TransportType::Tcp, 50.0, 0.0, 500_000.0);
        rotator.update_health(health);
        assert_eq!(rotator.all_health_reports().len(), 1);
        assert!(rotator.get_health(&TransportType::Tcp).is_some());
    }

    #[test]
    fn test_update_health_batch() {
        let mut rotator = TransportRotator::new();
        let reports = vec![
            make_health(TransportType::Tcp, 50.0, 0.0, 500_000.0),
            make_health(TransportType::Quic, 30.0, 0.0, 800_000.0),
            make_health(TransportType::WebSocket, 100.0, 0.02, 400_000.0),
        ];
        rotator.update_health_batch(reports);
        assert_eq!(rotator.all_health_reports().len(), 3);
    }

    #[test]
    fn test_select_best() {
        let mut rotator = TransportRotator::new();
        rotator.update_health_batch(vec![
            make_health(TransportType::Tcp, 200.0, 0.05, 300_000.0),
            make_health(TransportType::Quic, 30.0, 0.0, 900_000.0),
            make_health(TransportType::WebSocket, 150.0, 0.02, 500_000.0),
        ]);
        let best = rotator.select_best();
        assert_eq!(best, Some(TransportType::Quic));
    }

    #[test]
    fn test_rotate_to_best_healthy() {
        let mut rotator = TransportRotator::new();
        rotator.update_health_batch(vec![
            make_health(TransportType::Tcp, 500.0, 0.15, 100_000.0), // unhealthy
            make_health(TransportType::Quic, 30.0, 0.0, 900_000.0),  // best
            make_health(TransportType::WebSocket, 100.0, 0.02, 500_000.0),
        ]);
        let new_transport = rotator.rotate().unwrap();
        assert_eq!(new_transport, TransportType::Quic);
        assert_eq!(rotator.rotation_count(), 1);
    }

    #[test]
    fn test_rotate_fallback_cycle() {
        let mut rotator = TransportRotator::new();
        // No health reports — should fallback to cycling
        let new_transport = rotator.rotate().unwrap();
        assert_eq!(new_transport, TransportType::Quic); // Next after Tcp
        assert_eq!(rotator.rotation_count(), 1);
    }

    #[test]
    fn test_force_transport() {
        let mut rotator = TransportRotator::new();
        rotator.force_transport(&TransportType::WebRtc).unwrap();
        assert_eq!(*rotator.current_transport(), TransportType::WebRtc);
        assert_eq!(rotator.rotation_count(), 1);
    }

    #[test]
    fn test_force_transport_not_available() {
        let config = RotatorConfig {
            active_protocols: vec![TransportType::Tcp],
            ..RotatorConfig::default()
        };
        let mut rotator = TransportRotator::with_config(config).unwrap();
        match rotator.force_transport(&TransportType::Quic) {
            Err(RotationError::TransportNotAvailable(t)) => assert_eq!(t, TransportType::Quic),
            other => panic!("Expected TransportNotAvailable, got {:?}", other),
        }
    }

    #[test]
    fn test_rotation_interval_with_jitter() {
        let mut rotator = TransportRotator::new();
        let interval1 = rotator.rotation_interval_with_jitter();
        let interval2 = rotator.rotation_interval_with_jitter();
        // Both should be >= base interval
        assert!(interval1 >= rotator.config().rotation_interval);
        assert!(interval2 >= rotator.config().rotation_interval);
        // Jitter should differ
        assert_ne!(interval1, interval2);
    }

    #[test]
    fn test_health_report_update() {
        let mut rotator = TransportRotator::new();
        let h1 = make_health(TransportType::Tcp, 50.0, 0.0, 500_000.0);
        let h2 = make_health(TransportType::Tcp, 100.0, 0.05, 300_000.0);
        rotator.update_health(h1);
        rotator.update_health(h2);
        // Should have only 1 report (updated)
        assert_eq!(rotator.all_health_reports().len(), 1);
        let report = rotator.get_health(&TransportType::Tcp).unwrap();
        assert_eq!(report.latency_ms, 100.0);
    }

    #[test]
    fn test_transport_display() {
        assert_eq!(format!("{}", TransportType::Tcp), "tcp");
        assert_eq!(format!("{}", TransportType::Quic), "quic");
        assert_eq!(format!("{}", TransportType::WebSocket), "websocket");
        assert_eq!(format!("{}", TransportType::WebRtc), "webrtc");
    }

    #[test]
    fn test_error_display() {
        match RotationError::NoHealthyTransport {
            e => assert!(!e.to_string().is_empty()),
        }
    }

    #[test]
    fn test_default() {
        let rotator = TransportRotator::default();
        assert_eq!(*rotator.current_transport(), TransportType::Tcp);
    }
}
