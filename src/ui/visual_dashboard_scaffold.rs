//! Visual Dashboard Scaffold \u2014 Sprint 83: The Empirical Strike & Visual Proof
//!
//! WebSocket/HTTP endpoint scaffold for streaming SAE activations and
//! topological metrics. WebGL placeholder for 3D manifold visualization.
//! Public metrics API for empirical validation.

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

// â”€â”€â”€ Errors â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone)]
pub enum DashboardError {
    PortInUse(u16),
    InvalidPort(u16),
    NoActiveStream,
    SerializationFailed(String),
    ConnectionLimitReached(usize),
}

impl std::error::Error for DashboardError {}

impl fmt::Display for DashboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DashboardError::PortInUse(port) => write!(f, "Port {} is already in use", port),
            DashboardError::InvalidPort(port) => write!(f, "Invalid port: {}", port),
            DashboardError::NoActiveStream => write!(f, "No active data stream"),
            DashboardError::SerializationFailed(msg) => {
                write!(f, "Serialization failed: {}", msg)
            }
            DashboardError::ConnectionLimitReached(limit) => {
                write!(f, "Connection limit reached: {}", limit)
            }
        }
    }
}

// â”€â”€â”€ Data Structures â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Single SAE activation point in 3D topological space.
#[derive(Debug, Clone)]
pub struct ActivationPoint {
    pub timestamp_ms: u64,
    pub node_id: String,
    pub semantic_axis: f64,
    pub cooperative_axis: f64,
    pub ethical_axis: f64,
    pub magnitude: f64,
}

impl ActivationPoint {
    pub fn new(
        timestamp_ms: u64,
        node_id: String,
        semantic_axis: f64,
        cooperative_axis: f64,
        ethical_axis: f64,
    ) -> Self {
        let magnitude = (semantic_axis * semantic_axis
            + cooperative_axis * cooperative_axis
            + ethical_axis * ethical_axis)
            .sqrt();
        Self {
            timestamp_ms,
            node_id,
            semantic_axis,
            cooperative_axis,
            ethical_axis,
            magnitude,
        }
    }

    /// Check if this point indicates divergence (ethical axis anomaly).
    pub fn is_divergent(&self, threshold: f64) -> bool {
        self.ethical_axis.abs() >= threshold
    }
}

impl fmt::Display for ActivationPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Activation(ts={}, node={}, x={:.3}, y={:.3}, z={:.3}, mag={:.3})",
            self.timestamp_ms,
            self.node_id,
            self.semantic_axis,
            self.cooperative_axis,
            self.ethical_axis,
            self.magnitude
        )
    }
}

/// Snapshot of the semantic manifold for 3D visualization.
#[derive(Debug, Clone)]
pub struct ManifoldData {
    pub timestamp_ms: u64,
    pub points: Vec<ActivationPoint>,
    pub centroid_x: f64,
    pub centroid_y: f64,
    pub centroid_z: f64,
    pub spread: f64,
    pub divergence_count: usize,
}

impl ManifoldData {
    pub fn new(timestamp_ms: u64) -> Self {
        Self {
            timestamp_ms,
            points: Vec::new(),
            centroid_x: 0.0,
            centroid_y: 0.0,
            centroid_z: 0.0,
            spread: 0.0,
            divergence_count: 0,
        }
    }

    pub fn add_point(&mut self, point: ActivationPoint, divergence_threshold: f64) {
        self.points.push(point);
        self.recalculate_centroid();
        self.divergence_count = self
            .points
            .iter()
            .filter(|p| p.is_divergent(divergence_threshold))
            .count();
    }

    fn recalculate_centroid(&mut self) {
        if self.points.is_empty() {
            return;
        }
        let n = self.points.len() as f64;
        self.centroid_x = self.points.iter().map(|p| p.semantic_axis).sum::<f64>() / n;
        self.centroid_y = self.points.iter().map(|p| p.cooperative_axis).sum::<f64>() / n;
        self.centroid_z = self.points.iter().map(|p| p.ethical_axis).sum::<f64>() / n;
        self.spread = self.compute_spread();
    }

    fn compute_spread(&self) -> f64 {
        if self.points.len() < 2 {
            return 0.0;
        }
        let mut total_dist = 0.0;
        let n = self.points.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let dx = self.points[i].semantic_axis - self.points[j].semantic_axis;
                let dy = self.points[i].cooperative_axis - self.points[j].cooperative_axis;
                let dz = self.points[i].ethical_axis - self.points[j].ethical_axis;
                total_dist += (dx * dx + dy * dy + dz * dz).sqrt();
            }
        }
        total_dist / (n * (n - 1) / 2) as f64
    }

    pub fn point_count(&self) -> usize {
        self.points.len()
    }
}

impl fmt::Display for ManifoldData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Manifold(ts={}, points={}, centroid=({:.3},{:.3},{:.3}), spread={:.3}, divergent={})",
            self.timestamp_ms,
            self.point_count(),
            self.centroid_x,
            self.centroid_y,
            self.centroid_z,
            self.spread,
            self.divergence_count
        )
    }
}

/// Server handle for the visual stream server.
#[derive(Debug)]
pub struct ServerHandle {
    pub port: u16,
    pub running: Arc<AtomicBool>,
    pub connections: Arc<AtomicU64>,
    pub endpoints: Vec<String>,
}

impl Clone for ServerHandle {
    fn clone(&self) -> Self {
        ServerHandle {
            port: self.port,
            running: Arc::clone(&self.running),
            connections: Arc::clone(&self.connections),
            endpoints: self.endpoints.clone(),
        }
    }
}

impl ServerHandle {
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn connection_count(&self) -> u64 {
        self.connections.load(Ordering::Relaxed)
    }
}

impl fmt::Display for ServerHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ServerHandle(port={}, running={}, connections={}, endpoints={})",
            self.port,
            self.is_running(),
            self.connection_count(),
            self.endpoints.len()
        )
    }
}

/// Dashboard configuration.
#[derive(Debug, Clone)]
pub struct DashboardConfig {
    pub port: u16,
    pub max_connections: usize,
    pub divergence_threshold: f64,
    pub buffer_size: usize,
    pub enable_websocket: bool,
    pub enable_http: bool,
}

impl DashboardConfig {
    pub fn default_topological() -> Self {
        Self {
            port: 8787,
            max_connections: 100,
            divergence_threshold: 2.0,
            buffer_size: 10000,
            enable_websocket: true,
            enable_http: true,
        }
    }

    pub fn validate(&self) -> Result<(), DashboardError> {
        if self.port == 0 {
            return Err(DashboardError::InvalidPort(self.port));
        }
        if self.max_connections == 0 {
            return Err(DashboardError::ConnectionLimitReached(0));
        }
        if self.buffer_size == 0 {
            return Err(DashboardError::SerializationFailed(
                "Buffer size must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self::default_topological()
    }
}

/// Visual dashboard scaffold for streaming SAE activations.
pub struct VisualDashboardScaffold {
    config: DashboardConfig,
    activations: Vec<ActivationPoint>,
    manifold: ManifoldData,
    server: Option<ServerHandle>,
    metrics: HashMap<String, f64>,
}

impl VisualDashboardScaffold {
    pub fn new() -> Self {
        Self {
            config: DashboardConfig::default(),
            activations: Vec::new(),
            manifold: ManifoldData::new(0),
            server: None,
            metrics: HashMap::new(),
        }
    }

    pub fn with_config(config: DashboardConfig) -> Result<Self, DashboardError> {
        config.validate()?;
        Ok(Self {
            config,
            activations: Vec::new(),
            manifold: ManifoldData::new(0),
            server: None,
            metrics: HashMap::new(),
        })
    }

    /// Start the visual stream server (scaffold - simulates server lifecycle).
    pub fn start_visual_stream_server(&mut self) -> Result<ServerHandle, DashboardError> {
        if self.server.as_ref().map_or(false, |s| s.is_running()) {
            return Err(DashboardError::PortInUse(self.config.port));
        }

        let mut endpoints = Vec::new();
        if self.config.enable_http {
            endpoints.push(format!("/http://0.0.0.0:{}/metrics", self.config.port));
            endpoints.push(format!("/http://0.0.0.0:{}/manifold", self.config.port));
        }
        if self.config.enable_websocket {
            endpoints.push(format!(
                "/ws://0.0.0.0:{}/stream/activations",
                self.config.port
            ));
        }

        let handle = ServerHandle {
            port: self.config.port,
            running: Arc::new(AtomicBool::new(true)),
            connections: Arc::new(AtomicU64::new(0)),
            endpoints,
        };

        self.server = Some(handle.clone());
        Ok(handle)
    }

    /// Stop the visual stream server.
    pub fn stop_server(&mut self) {
        if let Some(ref handle) = self.server {
            handle.stop();
        }
    }

    /// Record a new SAE activation.
    pub fn record_activation(&mut self, point: ActivationPoint) {
        self.activations.push(point.clone());
        self.manifold
            .add_point(point, self.config.divergence_threshold);

        // Enforce buffer limit
        if self.activations.len() > self.config.buffer_size {
            self.activations
                .drain(0..self.activations.len() - self.config.buffer_size);
        }
    }

    /// Get the current manifold snapshot.
    pub fn get_manifold_snapshot(&self) -> &ManifoldData {
        &self.manifold
    }

    /// Get recent activations (last N).
    pub fn get_recent_activations(&self, count: usize) -> &[ActivationPoint] {
        if count >= self.activations.len() {
            &self.activations
        } else {
            &self.activations[self.activations.len() - count..]
        }
    }

    /// Record a metric value.
    pub fn record_metric(&mut self, name: String, value: f64) {
        self.metrics.insert(name, value);
    }

    /// Get all metrics.
    pub fn get_metrics(&self) -> &HashMap<String, f64> {
        &self.metrics
    }

    /// Get a specific metric.
    pub fn get_metric(&self, name: &str) -> Option<f64> {
        self.metrics.get(name).copied()
    }

    /// Export activations as JSON array.
    pub fn export_activations_json(&self) -> String {
        let points: Vec<String> = self
            .activations
            .iter()
            .map(|p| {
                format!(
                    "{{\"ts\":{},\"node\":\"{}\",\"x\":{:.4},\"y\":{:.4},\"z\":{:.4},\"mag\":{:.4}}}",
                    p.timestamp_ms, p.node_id, p.semantic_axis, p.cooperative_axis, p.ethical_axis, p.magnitude
                )
            })
            .collect();
        format!("[{}]", points.join(","))
    }

    /// Export manifold snapshot as JSON.
    pub fn export_manifold_json(&self) -> String {
        format!(
            "{{\"ts\":{},\"points\":{},\"centroid\":{{\"x\":{:.4},\"y\":{:.4},\"z\":{:.4}}},\"spread\":{:.4},\"divergent\":{}}}",
            self.manifold.timestamp_ms,
            self.manifold.point_count(),
            self.manifold.centroid_x,
            self.manifold.centroid_y,
            self.manifold.centroid_z,
            self.manifold.spread,
            self.manifold.divergence_count
        )
    }

    /// Get activation count.
    pub fn activation_count(&self) -> usize {
        self.activations.len()
    }

    /// Reset dashboard state.
    pub fn reset(&mut self) {
        self.activations.clear();
        self.manifold = ManifoldData::new(0);
        self.metrics.clear();
        self.stop_server();
        self.server = None;
    }
}

impl Default for VisualDashboardScaffold {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for VisualDashboardScaffold {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VisualDashboard(port={}, activations={}, manifold={}, metrics={}, server={})",
            self.config.port,
            self.activation_count(),
            self.manifold.point_count(),
            self.metrics.len(),
            self.server
                .as_ref()
                .map_or("none".to_string(), |s| format!("{}", s))
        )
    }
}

// â”€â”€â”€ Standalone Functions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Start visual stream server (standalone for CLI integration).
pub fn start_visual_stream_server(port: u16) -> Result<ServerHandle, Box<dyn std::error::Error>> {
    if port == 0 {
        return Err(Box::new(DashboardError::InvalidPort(port)));
    }
    let config = DashboardConfig {
        port,
        ..Default::default()
    };
    let mut scaffold = VisualDashboardScaffold::with_config(config)?;
    let handle = scaffold.start_visual_stream_server()?;
    Ok(handle)
}

/// Get manifold snapshot (standalone placeholder).
pub fn get_manifold_snapshot() -> ManifoldData {
    ManifoldData::new(0)
}

// â”€â”€â”€ Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = DashboardConfig::default_topological();
        assert_eq!(config.port, 8787);
        assert_eq!(config.max_connections, 100);
        assert_eq!(config.divergence_threshold, 2.0);
        assert!(config.enable_websocket);
        assert!(config.enable_http);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = DashboardConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_port_zero() {
        let config = DashboardConfig {
            port: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_connections() {
        let config = DashboardConfig {
            max_connections: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_buffer() {
        let config = DashboardConfig {
            buffer_size: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_activation_new() {
        let p = ActivationPoint::new(1000, "node1".to_string(), 1.0, 2.0, 3.0);
        assert_eq!(p.timestamp_ms, 1000);
        assert!((p.magnitude - 3.7417).abs() < 0.01);
    }

    #[test]
    fn test_activation_divergent() {
        let p = ActivationPoint::new(0, "n".to_string(), 0.0, 0.0, 3.0);
        assert!(p.is_divergent(2.0));
        assert!(!p.is_divergent(5.0));
    }

    #[test]
    fn test_activation_display() {
        let p = ActivationPoint::new(0, "n".to_string(), 0.0, 0.0, 0.0);
        let display = format!("{}", p);
        assert!(display.contains("Activation"));
    }

    #[test]
    fn test_manifold_new() {
        let m = ManifoldData::new(100);
        assert_eq!(m.timestamp_ms, 100);
        assert_eq!(m.point_count(), 0);
    }

    #[test]
    fn test_manifold_add_point() {
        let mut m = ManifoldData::new(0);
        let p = ActivationPoint::new(0, "n".to_string(), 1.0, 2.0, 3.0);
        m.add_point(p, 2.0);
        assert_eq!(m.point_count(), 1);
    }

    #[test]
    fn test_manifold_centroid() {
        let mut m = ManifoldData::new(0);
        m.add_point(ActivationPoint::new(0, "a".to_string(), 1.0, 0.0, 0.0), 5.0);
        m.add_point(ActivationPoint::new(0, "b".to_string(), 3.0, 0.0, 0.0), 5.0);
        assert!((m.centroid_x - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_manifold_divergence_count() {
        let mut m = ManifoldData::new(0);
        m.add_point(ActivationPoint::new(0, "a".to_string(), 0.0, 0.0, 3.0), 2.0);
        m.add_point(ActivationPoint::new(0, "b".to_string(), 0.0, 0.0, 0.5), 2.0);
        assert_eq!(m.divergence_count, 1);
    }

    #[test]
    fn test_manifold_display() {
        let m = ManifoldData::new(0);
        let display = format!("{}", m);
        assert!(display.contains("Manifold"));
    }

    #[test]
    fn test_server_handle() {
        let h = start_visual_stream_server(9999).unwrap();
        assert!(h.is_running());
        assert_eq!(h.port, 9999);
        h.stop();
        assert!(!h.is_running());
    }

    #[test]
    fn test_server_handle_display() {
        let h = start_visual_stream_server(9998).unwrap();
        let display = format!("{}", h);
        assert!(display.contains("ServerHandle"));
        h.stop();
    }

    #[test]
    fn test_engine_creation() {
        let engine = VisualDashboardScaffold::new();
        assert_eq!(engine.activation_count(), 0);
        assert!(engine.get_metrics().is_empty());
    }

    #[test]
    fn test_engine_with_config() {
        let config = DashboardConfig {
            port: 7777,
            ..Default::default()
        };
        let engine = VisualDashboardScaffold::with_config(config).unwrap();
        assert_eq!(engine.config.port, 7777);
    }

    #[test]
    fn test_start_server() {
        let mut engine = VisualDashboardScaffold::new();
        let handle = engine.start_visual_stream_server().unwrap();
        assert!(handle.is_running());
        assert_eq!(handle.port, 8787);
    }

    #[test]
    fn test_start_server_duplicate() {
        let mut engine = VisualDashboardScaffold::new();
        engine.start_visual_stream_server().unwrap();
        let result = engine.start_visual_stream_server();
        assert!(result.is_err());
    }

    #[test]
    fn test_stop_server() {
        let mut engine = VisualDashboardScaffold::new();
        let handle = engine.start_visual_stream_server().unwrap();
        engine.stop_server();
        assert!(!handle.is_running());
    }

    #[test]
    fn test_record_activation() {
        let mut engine = VisualDashboardScaffold::new();
        let p = ActivationPoint::new(100, "n1".to_string(), 1.0, 2.0, 3.0);
        engine.record_activation(p);
        assert_eq!(engine.activation_count(), 1);
    }

    #[test]
    fn test_get_recent_activations() {
        let mut engine = VisualDashboardScaffold::new();
        for i in 0..5 {
            engine.record_activation(ActivationPoint::new(i, "n".to_string(), 0.0, 0.0, 0.0));
        }
        let recent = engine.get_recent_activations(3);
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_record_metric() {
        let mut engine = VisualDashboardScaffold::new();
        engine.record_metric("tcm_z".to_string(), 2.5);
        assert_eq!(engine.get_metric("tcm_z"), Some(2.5));
        assert_eq!(engine.get_metric("missing"), None);
    }

    #[test]
    fn test_export_activations_json() {
        let mut engine = VisualDashboardScaffold::new();
        engine.record_activation(ActivationPoint::new(100, "n".to_string(), 1.0, 0.0, 0.0));
        let json = engine.export_activations_json();
        assert!(json.contains("\"ts\":100"));
        assert!(json.starts_with("["));
    }

    #[test]
    fn test_export_manifold_json() {
        let engine = VisualDashboardScaffold::new();
        let json = engine.export_manifold_json();
        assert!(json.contains("\"ts\":0"));
        assert!(json.contains("\"points\":0"));
    }

    #[test]
    fn test_reset() {
        let mut engine = VisualDashboardScaffold::new();
        engine.record_activation(ActivationPoint::new(0, "n".to_string(), 0.0, 0.0, 0.0));
        engine.record_metric("m".to_string(), 1.0);
        engine.start_visual_stream_server().unwrap();
        engine.reset();
        assert_eq!(engine.activation_count(), 0);
        assert!(engine.get_metrics().is_empty());
        assert!(engine.server.is_none());
    }

    #[test]
    fn test_display() {
        let engine = VisualDashboardScaffold::new();
        let display = format!("{}", engine);
        assert!(display.contains("VisualDashboard"));
    }

    #[test]
    fn test_standalone_start_server() {
        let handle = start_visual_stream_server(8888).unwrap();
        assert!(handle.is_running());
        assert_eq!(handle.port, 8888);
        handle.stop();
    }

    #[test]
    fn test_standalone_start_server_invalid_port() {
        let result = start_visual_stream_server(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_standalone_get_manifold_snapshot() {
        let m = get_manifold_snapshot();
        assert_eq!(m.point_count(), 0);
    }

    #[test]
    fn test_buffer_limit_enforced() {
        let config = DashboardConfig {
            buffer_size: 3,
            ..Default::default()
        };
        let mut engine = VisualDashboardScaffold::with_config(config).unwrap();
        for i in 0..10 {
            engine.record_activation(ActivationPoint::new(i, "n".to_string(), 0.0, 0.0, 0.0));
        }
        assert!(engine.activation_count() <= 3);
    }

    #[test]
    fn test_error_display() {
        let err = DashboardError::PortInUse(8080);
        assert!(format!("{}", err).contains("8080"));
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = VisualDashboardScaffold::new();
        let handle = engine.start_visual_stream_server().unwrap();
        assert!(handle.is_running());

        for i in 0..5 {
            engine.record_activation(ActivationPoint::new(
                i * 100,
                format!("node{}", i),
                i as f64,
                (i * 2) as f64,
                (i * 3) as f64,
            ));
        }
        assert_eq!(engine.activation_count(), 5);

        let manifold = engine.get_manifold_snapshot();
        assert_eq!(manifold.point_count(), 5);

        engine.record_metric("tcm_z_avg".to_string(), 1.5);
        assert_eq!(engine.get_metric("tcm_z_avg"), Some(1.5));

        let json = engine.export_activations_json();
        assert!(json.contains("node0"));

        let mjson = engine.export_manifold_json();
        assert!(mjson.contains("points\":5"));

        engine.stop_server();
        assert!(!handle.is_running());
    }
}
