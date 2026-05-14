//! WebSocket Alignment Stream — Streaming de señales de alineación vía WebSocket (Sprint 4)
//!
//! LP-33: Real-time UI/Backend
//! Proporciona streaming en tiempo real de señales de alineación (feedback, steering, drift)
//! a clientes WebSocket con filtrado por tipo, rate limiting y reconexión automática.
//!
//! Características:
//! - Streaming de señales de Alignment Loop v2
//! - Filtrado por tipo de señal (feedback, steering, drift, cycle)
//! - Rate limiting por conexión con ventanas deslizantes
//! - Manejo de reconexión con last-seen sequence
//! - Formato SSE-compatible para interoperabilidad
//! - Estadísticas de latencia y throughput por conexión
//!
//! Protegido con `#[cfg(feature = "v1.1-sprint4")]`.

#[cfg(feature = "v1.1-sprint4")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.1-sprint4")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "v1.1-sprint4")]
use std::time::{Duration, Instant};
#[cfg(feature = "v1.1-sprint4")]
use thiserror::Error;
#[cfg(feature = "v1.1-sprint4")]
use tracing::info;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
#[derive(Error, Debug)]
pub enum WsAlignmentError {
    #[error("Conexión no encontrada: {0}")]
    ConnectionNotFound(String),

    #[error("Rate limit excedido: {current}/{max} msg/s")]
    RateLimitExceeded { current: usize, max: usize },

    #[error("Tipo de señal inválido: {0}")]
    InvalidSignalType(String),

    #[error("Conexión ya existe: {0}")]
    ConnectionAlreadyExists(String),

    #[error("Máximo de conexiones alcanzado: {0}")]
    MaxConnectionsReached(usize),

    #[error("Secuencia inválida: expected {expected}, got {got}")]
    InvalidSequence { expected: u64, got: u64 },

    #[error("Error de serialización: {0}")]
    SerializationError(String),
}

// ─── Signal Types ─────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentSignalType {
    /// Feedback de anotador verificado criptográficamente
    Feedback,
    /// Señal de steering generada por Loop v2
    Steering,
    /// Medición de drift de alineación
    Drift,
    /// Ciclo de alineación completado
    CycleComplete,
    /// Señal de rollback por degradación
    Rollback,
    /// Métricas de confianza ponderada
    Confidence,
}

#[cfg(feature = "v1.1-sprint4")]
impl std::fmt::Display for AlignmentSignalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlignmentSignalType::Feedback => write!(f, "alignment_feedback"),
            AlignmentSignalType::Steering => write!(f, "alignment_steering"),
            AlignmentSignalType::Drift => write!(f, "alignment_drift"),
            AlignmentSignalType::CycleComplete => write!(f, "alignment_cycle"),
            AlignmentSignalType::Rollback => write!(f, "alignment_rollback"),
            AlignmentSignalType::Confidence => write!(f, "alignment_confidence"),
        }
    }
}

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentSignal {
    pub signal_id: String,
    pub signal_type: AlignmentSignalType,
    pub timestamp_ms: u64,
    pub layer_id: String,
    pub payload: serde_json::Value,
    pub sequence: u64,
    pub source_node: Option<String>,
    pub confidence: f32,
}

#[cfg(feature = "v1.1-sprint4")]
impl AlignmentSignal {
    pub fn new(
        signal_type: AlignmentSignalType,
        layer_id: String,
        payload: serde_json::Value,
        sequence: u64,
        source_node: Option<String>,
        confidence: f32,
    ) -> Self {
        Self {
            signal_id: format!(
                "sig-{}-{}-{}",
                signal_type,
                current_timestamp_ms(),
                sequence
            ),
            signal_type,
            timestamp_ms: current_timestamp_ms(),
            layer_id,
            payload,
            sequence,
            source_node,
            confidence,
        }
    }
}

// ─── Connection State ─────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone)]
pub struct WsAlignmentConnection {
    pub connection_id: String,
    pub subscribed_types: Vec<AlignmentSignalType>,
    pub connected_at: Instant,
    pub last_activity: Instant,
    pub last_sequence_seen: u64,
    pub messages_sent: usize,
    pub messages_received: usize,
    pub rate_limit_per_sec: usize,
    pub current_window_count: usize,
    pub window_start: Instant,
    pub pending_signals: VecDeque<AlignmentSignal>,
    pub max_buffer_size: usize,
    pub active: bool,
    pub reconnect_token: Option<String>,
}

#[cfg(feature = "v1.1-sprint4")]
impl WsAlignmentConnection {
    pub fn new(
        connection_id: String,
        subscribed_types: Vec<AlignmentSignalType>,
        rate_limit_per_sec: usize,
    ) -> Self {
        Self {
            connection_id: connection_id.clone(),
            subscribed_types,
            connected_at: Instant::now(),
            last_activity: Instant::now(),
            last_sequence_seen: 0,
            messages_sent: 0,
            messages_received: 0,
            rate_limit_per_sec,
            current_window_count: 0,
            window_start: Instant::now(),
            pending_signals: VecDeque::new(),
            max_buffer_size: 500,
            active: true,
            reconnect_token: Some(generate_reconnect_token(&connection_id)),
        }
    }

    /// Verificar rate limit.
    pub fn check_rate_limit(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.window_start).as_secs() >= 1 {
            self.current_window_count = 0;
            self.window_start = now;
        }
        self.current_window_count < self.rate_limit_per_sec
    }

    /// Registrar mensaje enviado.
    pub fn record_message(&mut self) {
        self.messages_sent += 1;
        self.current_window_count += 1;
        self.last_activity = Instant::now();
    }

    /// Buffer signal con backpressure.
    pub fn buffer_signal(&mut self, signal: AlignmentSignal) -> bool {
        if self.pending_signals.len() < self.max_buffer_size {
            self.pending_signals.push_back(signal);
            true
        } else {
            false
        }
    }

    /// Verificar si suscribe al tipo.
    pub fn is_subscribed(&self, signal_type: &AlignmentSignalType) -> bool {
        self.subscribed_types.contains(signal_type)
    }

    /// Obtener señales pendientes desde última secuencia.
    pub fn get_pending_since(&mut self, since_sequence: u64) -> Vec<AlignmentSignal> {
        let mut result = Vec::new();
        while let Some(signal) = self.pending_signals.front() {
            if signal.sequence > since_sequence {
                result.push(self.pending_signals.pop_front().unwrap());
            } else if signal.sequence <= since_sequence {
                // Saltar señales ya vistas
                self.pending_signals.pop_front();
            } else {
                break;
            }
        }
        result
    }

    /// Verificar si expiró.
    pub fn is_expired(&self, max_age_secs: u64) -> bool {
        self.connected_at.elapsed().as_secs() > max_age_secs
    }

    /// Desactivar conexión.
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

// ─── Stream Result ────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsStreamResult {
    pub connection_id: String,
    pub signals_sent: usize,
    pub signals_buffered: usize,
    pub signals_dropped: usize,
    pub rate_limited: bool,
    pub active_connections: usize,
    pub last_sequence: u64,
}

#[cfg(feature = "v1.1-sprint4")]
impl WsStreamResult {
    pub fn success(
        connection_id: String,
        signals_sent: usize,
        signals_buffered: usize,
        active_connections: usize,
        last_sequence: u64,
    ) -> Self {
        Self {
            connection_id,
            signals_sent,
            signals_buffered,
            signals_dropped: 0,
            rate_limited: false,
            active_connections,
            last_sequence,
        }
    }
}

// ─── Config ───────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsAlignmentConfig {
    pub max_connections: usize,
    pub rate_limit_per_sec: usize,
    pub max_connection_age_secs: u64,
    pub signal_history_size: usize,
    pub enable_backpressure: bool,
    pub reconnect_window_secs: u64,
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for WsAlignmentConfig {
    fn default() -> Self {
        Self {
            max_connections: 50,
            rate_limit_per_sec: 200,
            max_connection_age_secs: 1800,
            signal_history_size: 3000,
            enable_backpressure: true,
            reconnect_window_secs: 300,
        }
    }
}

// ─── Stats ────────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsAlignmentStats {
    pub active_connections: usize,
    pub total_signals_sent: u64,
    pub total_signals_dropped: u64,
    pub total_rate_limited: u64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub signals_per_second: f64,
    pub reconnects: u64,
    pub last_updated_ms: u64,
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for WsAlignmentStats {
    fn default() -> Self {
        Self {
            active_connections: 0,
            total_signals_sent: 0,
            total_signals_dropped: 0,
            total_rate_limited: 0,
            avg_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            signals_per_second: 0.0,
            reconnects: 0,
            last_updated_ms: 0,
        }
    }
}

// ─── WsAlignmentStream ────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint4")]
pub struct WsAlignmentStream {
    pub config: WsAlignmentConfig,
    pub connections: HashMap<String, WsAlignmentConnection>,
    pub signal_history: VecDeque<AlignmentSignal>,
    pub stats: WsAlignmentStats,
    pub sequence_counter: u64,
    pub latency_samples: VecDeque<f64>,
    pub reconnect_tokens: HashMap<String, Instant>,
}

#[cfg(feature = "v1.1-sprint4")]
impl WsAlignmentStream {
    pub fn new() -> Self {
        Self {
            config: WsAlignmentConfig::default(),
            connections: HashMap::new(),
            signal_history: VecDeque::new(),
            stats: WsAlignmentStats::default(),
            sequence_counter: 0,
            latency_samples: VecDeque::new(),
            reconnect_tokens: HashMap::new(),
        }
    }

    pub fn with_config(config: WsAlignmentConfig) -> Self {
        Self {
            config,
            connections: HashMap::new(),
            signal_history: VecDeque::new(),
            stats: WsAlignmentStats::default(),
            sequence_counter: 0,
            latency_samples: VecDeque::new(),
            reconnect_tokens: HashMap::new(),
        }
    }

    /// Crear nueva conexión WebSocket.
    pub fn create_connection(
        &mut self,
        connection_id: String,
        subscribed_types: Vec<AlignmentSignalType>,
    ) -> Result<WsAlignmentConnection, WsAlignmentError> {
        if self.connections.contains_key(&connection_id) {
            return Err(WsAlignmentError::ConnectionAlreadyExists(connection_id));
        }

        if self.connections.len() >= self.config.max_connections {
            return Err(WsAlignmentError::MaxConnectionsReached(
                self.config.max_connections,
            ));
        }

        let connection = WsAlignmentConnection::new(
            connection_id.clone(),
            subscribed_types,
            self.config.rate_limit_per_sec,
        );

        // Guardar token de reconexión
        if let Some(token) = &connection.reconnect_token {
            self.reconnect_tokens
                .insert(token.clone(), Instant::now());
        }

        self.connections.insert(connection_id.clone(), connection);
        info!(
            "Conexión WS creada: {} ({} conexiones activas)",
            connection_id,
            self.connections.len()
        );

        Ok(self.connections.get(&connection_id).unwrap().clone())
    }

    /// Reconectar con token.
    pub fn reconnect_with_token(
        &mut self,
        connection_id: String,
        token: String,
        last_sequence: u64,
    ) -> Result<WsAlignmentConnection, WsAlignmentError> {
        // Verificar token
        match self.reconnect_tokens.get(&token) {
            Some(created_at) => {
                if created_at.elapsed().as_secs() > self.config.reconnect_window_secs {
                    return Err(WsAlignmentError::ConnectionNotFound(
                        "Token expirado".to_string(),
                    ));
                }
            }
            None => {
                return Err(WsAlignmentError::ConnectionNotFound(
                    "Token inválido".to_string(),
                ));
            }
        }

        self.stats.reconnects += 1;

        // Crear nueva conexión con tipos por defecto
        let types = vec![
            AlignmentSignalType::Feedback,
            AlignmentSignalType::Steering,
            AlignmentSignalType::Drift,
        ];
        let mut connection = self.create_connection(connection_id.clone(), types)?;
        connection.last_sequence_seen = last_sequence;

        info!(
            "Reconexión exitosa: {} desde secuencia {}",
            connection_id, last_sequence
        );

        Ok(connection)
    }

    /// Cerrar conexión.
    pub fn close_connection(
        &mut self,
        connection_id: &str,
    ) -> Result<(), WsAlignmentError> {
        match self.connections.remove(connection_id) {
            Some(_) => {
                info!("Conexión cerrada: {}", connection_id);
                Ok(())
            }
            None => Err(WsAlignmentError::ConnectionNotFound(
                connection_id.to_string(),
            )),
        }
    }

    /// Emitir señal de alineación a todas las conexiones suscritas.
    pub fn emit_signal(
        &mut self,
        signal_type: AlignmentSignalType,
        layer_id: String,
        payload: serde_json::Value,
        source_node: Option<String>,
        confidence: f32,
    ) -> WsStreamResult {
        self.sequence_counter += 1;
        let signal = AlignmentSignal::new(
            signal_type.clone(),
            layer_id,
            payload,
            self.sequence_counter,
            source_node,
            confidence,
        );

        let start = Instant::now();
        let mut signals_sent = 0;
        let mut signals_buffered = 0;
        let mut _signals_dropped = 0;
        let mut _rate_limited = false;

        for connection in self.connections.values_mut() {
            if !connection.active || !connection.is_subscribed(&signal.signal_type) {
                continue;
            }

            if !connection.check_rate_limit() {
                _rate_limited = true;
                self.stats.total_rate_limited += 1;

                if self.config.enable_backpressure {
                    if connection.buffer_signal(signal.clone()) {
                        signals_buffered += 1;
                    } else {
                        _signals_dropped += 1;
                        self.stats.total_signals_dropped += 1;
                    }
                } else {
                    _signals_dropped += 1;
                    self.stats.total_signals_dropped += 1;
                }
                continue;
            }

            connection.record_message();
            connection.last_sequence_seen = signal.sequence;
            signals_sent += 1;
        }

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        self.latency_samples.push_back(elapsed_ms);
        if self.latency_samples.len() > 1000 {
            self.latency_samples.pop_front();
        }

        // Guardar en historial
        self.signal_history.push_back(signal);
        while self.signal_history.len() > self.config.signal_history_size {
            self.signal_history.pop_front();
        }

        self.stats.total_signals_sent += signals_sent as u64;
        self.update_stats();

        WsStreamResult::success(
            "broadcast".to_string(),
            signals_sent,
            signals_buffered,
            self.connections.len(),
            self.sequence_counter,
        )
    }

    /// Obtener señales de catch-up para reconexión.
    pub fn get_catchup_signals(
        &self,
        connection_id: &str,
        since_sequence: u64,
    ) -> Vec<AlignmentSignal> {
        let connection = match self.connections.get(connection_id) {
            Some(c) => c,
            None => return Vec::new(),
        };

        self.signal_history
            .iter()
            .filter(|s| s.sequence > since_sequence && connection.is_subscribed(&s.signal_type))
            .cloned()
            .collect()
    }

    /// Formatear señal como evento SSE.
    pub fn format_sse_signal(signal: &AlignmentSignal) -> String {
        format!(
            "event: {}\ndata: {}\nid: {}\ntimestamp: {}\n\n",
            signal.signal_type,
            serde_json::to_string(signal).unwrap_or_default(),
            signal.signal_id,
            signal.timestamp_ms,
        )
    }

    /// Limpiar conexiones expiradas.
    pub fn cleanup_expired_connections(&mut self) -> usize {
        let before = self.connections.len();
        self.connections
            .retain(|_, c| !c.is_expired(self.config.max_connection_age_secs));
        let removed = before - self.connections.len();
        if removed > 0 {
            info!("{} conexiones expiradas limpiadas", removed);
        }
        self.update_stats();
        removed
    }

    /// Limpiar tokens de reconexión expirados.
    pub fn cleanup_expired_tokens(&mut self) -> usize {
        let cutoff = Instant::now() - Duration::from_secs(self.config.reconnect_window_secs);
        self.reconnect_tokens
            .retain(|_, created_at| *created_at > cutoff);
        0
    }

    /// Obtener estadísticas.
    pub fn get_stats(&self) -> WsAlignmentStats {
        self.stats.clone()
    }

    /// Obtener conexión.
    pub fn get_connection(&self, connection_id: &str) -> Option<&WsAlignmentConnection> {
        self.connections.get(connection_id)
    }

    /// Reiniciar estadísticas.
    pub fn reset_stats(&mut self) {
        self.stats = WsAlignmentStats::default();
        self.latency_samples.clear();
    }

    /// Actualizar estadísticas.
    fn update_stats(&mut self) {
        self.stats.active_connections = self.connections.len();
        self.stats.last_updated_ms = current_timestamp_ms();

        if !self.latency_samples.is_empty() {
            let sum: f64 = self.latency_samples.iter().sum();
            self.stats.avg_latency_ms = sum / self.latency_samples.len() as f64;

            let mut sorted: Vec<f64> = self.latency_samples.iter().cloned().collect();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let p95_idx = ((sorted.len() as f64) * 0.95) as usize;
            self.stats.p95_latency_ms =
                *sorted.get(p95_idx.min(sorted.len() - 1)).unwrap_or(&0.0);
        }
    }
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for WsAlignmentStream {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "v1.1-sprint4")]
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(feature = "v1.1-sprint4")]
fn generate_reconnect_token(connection_id: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(format!("{}:{}", connection_id, current_timestamp_ms()));
    let result = hasher.finalize();
    hex::encode(&result[..16])
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(all(test, feature = "v1.1-sprint4"))]
mod tests {
    use super::*;

    #[test]
    fn test_stream_creation() {
        let stream = WsAlignmentStream::new();
        assert_eq!(stream.connections.len(), 0);
        assert_eq!(stream.config.max_connections, 50);
    }

    #[test]
    fn test_stream_with_config() {
        let config = WsAlignmentConfig {
            max_connections: 25,
            rate_limit_per_sec: 500,
            ..WsAlignmentConfig::default()
        };
        let stream = WsAlignmentStream::with_config(config);
        assert_eq!(stream.config.max_connections, 25);
    }

    #[test]
    fn test_create_connection() {
        let mut stream = WsAlignmentStream::new();
        let types = vec![AlignmentSignalType::Feedback];
        let conn = stream.create_connection("c1".to_string(), types).unwrap();
        assert_eq!(conn.connection_id, "c1");
        assert_eq!(stream.connections.len(), 1);
    }

    #[test]
    fn test_create_duplicate_connection() {
        let mut stream = WsAlignmentStream::new();
        let types = vec![AlignmentSignalType::Feedback];
        stream.create_connection("c1".to_string(), types.clone()).unwrap();
        let result = stream.create_connection("c1".to_string(), types);
        assert!(result.is_err());
    }

    #[test]
    fn test_max_connections_reached() {
        let config = WsAlignmentConfig {
            max_connections: 1,
            ..WsAlignmentConfig::default()
        };
        let mut stream = WsAlignmentStream::with_config(config);
        let types = vec![AlignmentSignalType::Feedback];

        stream.create_connection("c1".to_string(), types.clone()).unwrap();
        let result = stream.create_connection("c2".to_string(), types);
        assert!(result.is_err());
    }

    #[test]
    fn test_close_connection() {
        let mut stream = WsAlignmentStream::new();
        let types = vec![AlignmentSignalType::Feedback];
        stream.create_connection("c1".to_string(), types).unwrap();
        stream.close_connection("c1").unwrap();
        assert_eq!(stream.connections.len(), 0);
    }

    #[test]
    fn test_close_unknown_connection() {
        let mut stream = WsAlignmentStream::new();
        let result = stream.close_connection("unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_emit_signal_to_subscribed() {
        let mut stream = WsAlignmentStream::new();
        let types = vec![AlignmentSignalType::Feedback];
        stream.create_connection("c1".to_string(), types).unwrap();

        let result = stream.emit_signal(
            AlignmentSignalType::Feedback,
            "layer_1".to_string(),
            serde_json::json!({"drift": 0.1}),
            Some("node1".to_string()),
            0.9,
        );
        assert_eq!(result.signals_sent, 1);
    }

    #[test]
    fn test_emit_signal_filtered() {
        let mut stream = WsAlignmentStream::new();
        let types = vec![AlignmentSignalType::Feedback];
        stream.create_connection("c1".to_string(), types).unwrap();

        let result = stream.emit_signal(
            AlignmentSignalType::Steering,
            "layer_1".to_string(),
            serde_json::json!({}),
            None,
            0.8,
        );
        assert_eq!(result.signals_sent, 0);
    }

    #[test]
    fn test_get_catchup_signals() {
        let mut stream = WsAlignmentStream::new();
        let types = vec![AlignmentSignalType::Feedback];
        stream.create_connection("c1".to_string(), types).unwrap();

        stream.emit_signal(
            AlignmentSignalType::Feedback,
            "layer_1".to_string(),
            serde_json::json!({"n": 1}),
            None,
            0.9,
        );
        stream.emit_signal(
            AlignmentSignalType::Feedback,
            "layer_1".to_string(),
            serde_json::json!({"n": 2}),
            None,
            0.9,
        );

        let catchup = stream.get_catchup_signals("c1", 0);
        assert_eq!(catchup.len(), 2);
    }

    #[test]
    fn test_format_sse_signal() {
        let signal = AlignmentSignal::new(
            AlignmentSignalType::Feedback,
            "layer_1".to_string(),
            serde_json::json!({}),
            1,
            None,
            0.9,
        );
        let sse = WsAlignmentStream::format_sse_signal(&signal);
        assert!(sse.contains("event: alignment_feedback"));
        assert!(sse.contains("data:"));
        assert!(sse.contains("id:"));
    }

    #[test]
    fn test_cleanup_expired_connections() {
        let mut stream = WsAlignmentStream::new();
        let types = vec![AlignmentSignalType::Feedback];
        stream.create_connection("c1".to_string(), types).unwrap();

        let conn = stream.connections.get_mut("c1").unwrap();
        conn.connected_at = Instant::now() - Duration::from_secs(3600);

        let removed = stream.cleanup_expired_connections();
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_stats_tracking() {
        let mut stream = WsAlignmentStream::new();
        let types = vec![AlignmentSignalType::Feedback];
        stream.create_connection("c1".to_string(), types).unwrap();

        stream.emit_signal(
            AlignmentSignalType::Feedback,
            "layer_1".to_string(),
            serde_json::json!({}),
            None,
            0.9,
        );

        let stats = stream.get_stats();
        assert_eq!(stats.active_connections, 1);
        assert!(stats.total_signals_sent > 0);
    }

    #[test]
    fn test_reset_stats() {
        let mut stream = WsAlignmentStream::new();
        stream.reset_stats();
        let stats = stream.get_stats();
        assert_eq!(stats.total_signals_sent, 0);
    }

    #[test]
    fn test_signal_type_display() {
        assert_eq!(
            AlignmentSignalType::Feedback.to_string(),
            "alignment_feedback"
        );
        assert_eq!(
            AlignmentSignalType::Steering.to_string(),
            "alignment_steering"
        );
    }

    #[test]
    fn test_config_default() {
        let config = WsAlignmentConfig::default();
        assert_eq!(config.max_connections, 50);
        assert_eq!(config.rate_limit_per_sec, 200);
        assert!(config.enable_backpressure);
    }

    #[test]
    fn test_stats_default() {
        let stats = WsAlignmentStats::default();
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.total_signals_sent, 0);
    }

    #[test]
    fn test_stream_default() {
        let stream = WsAlignmentStream::default();
        assert_eq!(stream.connections.len(), 0);
    }

    #[test]
    fn test_connection_buffer_signal() {
        let mut conn = WsAlignmentConnection::new("c1".to_string(), vec![], 100);
        let signal = AlignmentSignal::new(
            AlignmentSignalType::Feedback,
            "layer_1".to_string(),
            serde_json::json!({}),
            1,
            None,
            0.9,
        );
        assert!(conn.buffer_signal(signal));
        assert_eq!(conn.pending_signals.len(), 1);
    }

    #[test]
    fn test_connection_get_pending_since() {
        let mut conn = WsAlignmentConnection::new("c1".to_string(), vec![], 100);
        for i in 1..=5 {
            conn.pending_signals.push_back(AlignmentSignal::new(
                AlignmentSignalType::Feedback,
                "layer_1".to_string(),
                serde_json::json!({}),
                i,
                None,
                0.9,
            ));
        }

        let pending = conn.get_pending_since(2);
        assert_eq!(pending.len(), 3); // secuencias 3, 4, 5
        assert_eq!(pending[0].sequence, 3);
    }

    #[test]
    fn test_multiple_connections_different_subscriptions() {
        let mut stream = WsAlignmentStream::new();
        stream
            .create_connection(
                "c1".to_string(),
                vec![AlignmentSignalType::Feedback],
            )
            .unwrap();
        stream
            .create_connection(
                "c2".to_string(),
                vec![AlignmentSignalType::Steering],
            )
            .unwrap();

        let result = stream.emit_signal(
            AlignmentSignalType::Feedback,
            "layer_1".to_string(),
            serde_json::json!({}),
            None,
            0.9,
        );
        assert_eq!(result.signals_sent, 1); // Solo c1
    }

    #[test]
    fn test_reconnect_token_generated() {
        let mut stream = WsAlignmentStream::new();
        let types = vec![AlignmentSignalType::Feedback];
        let conn = stream.create_connection("c1".to_string(), types).unwrap();
        assert!(conn.reconnect_token.is_some());
        assert!(!stream.reconnect_tokens.is_empty());
    }
}
