//! Web Server - Axum/Tower HTTP server + rutas API
//!
//! Servidor HTTP embebido en el mismo binario para dashboard de monitoreo,
//! feedback y métricas. Usa `axum` + `tower-http` para servir API REST
//! y archivos estáticos del frontend.

use std::net::SocketAddr;
use std::sync::Arc;
// CLEANUP: removed unused imports Duration, State, StatusCode, Deserialize, Serialize, warn
use std::time::Instant;

use axum::routing::{get, post};
use axum::Router;
// CLEANUP: removed unused import serde::Serialize
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::info;

use super::routes::{
    get_feedback, get_health, get_metrics, get_network, get_status, handle_feedback,
};

/// Estado compartido del servidor web
#[derive(Clone)]
pub struct WebServerState {
    /// Timestamp de inicio del servidor
    pub start_time: Instant,
    /// Ruta al directorio de archivos estáticos
    pub static_dir: String,
    /// Callback para obtener estado del nodo (evita acoplar directamente)
    pub node_status_fn: Arc<dyn Fn() -> serde_json::Value + Send + Sync>,
    /// Callback para obtener info de red
    pub network_info_fn: Arc<dyn Fn() -> serde_json::Value + Send + Sync>,
    /// Callback para obtener métricas
    pub metrics_fn: Arc<dyn Fn() -> String + Send + Sync>,
    /// Callback para obtener feedback
    pub feedback_fn: Arc<dyn Fn() -> serde_json::Value + Send + Sync>,
    /// Callback para recibir feedback
    pub submit_feedback_fn: Arc<dyn Fn(serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync>,
    /// Callback para health check
    pub health_fn: Arc<dyn Fn() -> (bool, String) + Send + Sync>,
}

impl WebServerState {
    pub fn new(
        static_dir: String,
        node_status_fn: Arc<dyn Fn() -> serde_json::Value + Send + Sync>,
        network_info_fn: Arc<dyn Fn() -> serde_json::Value + Send + Sync>,
        metrics_fn: Arc<dyn Fn() -> String + Send + Sync>,
        feedback_fn: Arc<dyn Fn() -> serde_json::Value + Send + Sync>,
        submit_feedback_fn: Arc<dyn Fn(serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync>,
        health_fn: Arc<dyn Fn() -> (bool, String) + Send + Sync>,
    ) -> Self {
        Self {
            start_time: Instant::now(),
            static_dir,
            node_status_fn,
            network_info_fn,
            metrics_fn,
            feedback_fn,
            submit_feedback_fn,
            health_fn,
        }
    }

    /// Crea estado con valores por defecto (para testing)
    pub fn default_state() -> Self {
        let empty_json = Arc::new(|| serde_json::json!({}));
        let empty_string = Arc::new(String::new); // CLEANUP: redundant closure
        let ok_feedback = Arc::new(|_| Ok(serde_json::json!({"status": "ok"})));
        let healthy = Arc::new(|| (true, "ok".to_string()));

        Self {
            start_time: Instant::now(),
            static_dir: "web".to_string(),
            node_status_fn: empty_json.clone(),
            network_info_fn: empty_json.clone(),
            metrics_fn: empty_string.clone(),
            feedback_fn: empty_json.clone(),
            submit_feedback_fn: ok_feedback.clone(),
            health_fn: healthy.clone(),
        }
    }

    /// Calcula uptime en segundos
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

/// Configuración del servidor web
#[derive(Debug, Clone)]
pub struct WebServerConfig {
    /// Dirección de bind (ej: "0.0.0.0:8080")
    pub bind_address: String,
    /// Ruta al directorio de archivos estáticos
    pub static_dir: String,
    /// Habilitar CORS
    pub enable_cors: bool,
    /// Timeout de requests en segundos
    pub request_timeout_secs: u64,
}

impl Default for WebServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0:8080".to_string(),
            static_dir: "web".to_string(),
            enable_cors: true,
            request_timeout_secs: 30,
        }
    }
}

/// Servidor web principal
pub struct WebServer {
    /// Configuración
    config: WebServerConfig,
    /// Estado compartido
    state: WebServerState,
}

impl WebServer {
    pub fn new(
        config: WebServerConfig,
        state: WebServerState,
    ) -> Self {
        Self { config, state }
    }

    /// Crea servidor con configuración por defecto
    pub fn default_server() -> Self {
        Self {
            config: WebServerConfig::default(),
            state: WebServerState::default_state(),
        }
    }

    /// Construye el router de axum
    fn build_router(state: WebServerState) -> Router {
        // API routes
        let api_router = Router::new()
            .route("/status", get(get_status))
            .route("/network", get(get_network))
            .route("/feedback", get(get_feedback))
            .route("/feedback", post(handle_feedback))
            .route("/metrics", get(get_metrics))
            .route("/health", get(get_health))
            // FIX: E0382 - clone state before it's moved into with_state()
            .with_state(state.clone());

        // Main router con static files
        Router::new()
            .nest("/api", api_router)
            .fallback_service(
                // FIX: E0599 - append_index_html_on_director → append_index_html_on_directories in axum
                ServeDir::new(&state.static_dir)
                    .append_index_html_on_directories(true),
            )
            .layer(TraceLayer::new_for_http())
    }

    /// Inicia el servidor web (async)
    pub async fn start(self) -> Result<(), WebServerError> {
        let app = Self::build_router(self.state.clone());

        let addr: SocketAddr = self
            .config
            .bind_address
            .parse()
            .map_err(|e| WebServerError::BindError(format!("Invalid address: {}", e)))?;

        info!(
            address = %addr,
            static_dir = %self.config.static_dir,
            "Starting web server"
        );

        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| WebServerError::BindError(format!("Failed to bind: {}", e)))?;

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(|e| WebServerError::ServerError(e.to_string()))?;

        Ok(())
    }

    /// Obtiene la dirección de bind
    pub fn bind_address(&self) -> &str {
        &self.config.bind_address
    }

    /// Obtiene el estado del servidor
    pub fn state(&self) -> &WebServerState {
        &self.state
    }
}

/// Señal de shutdown graceful
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received, starting graceful shutdown");
}

/// Errores del servidor web
#[derive(Debug, thiserror::Error)]
pub enum WebServerError {
    #[error("Bind error: {0}")]
    BindError(String),
    #[error("Server error: {0}")]
    ServerError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_server_config_default() {
        let config = WebServerConfig::default();
        assert_eq!(config.bind_address, "0.0.0.0:8080");
        assert_eq!(config.static_dir, "web");
        assert!(config.enable_cors);
    }

    #[test]
    fn test_web_server_state_default() {
        let state = WebServerState::default_state();
        assert_eq!(state.uptime_seconds(), 0);
    }

    #[test]
    fn test_web_server_creation() {
        let server = WebServer::default_server();
        assert_eq!(server.bind_address(), "0.0.0.0:8080");
    }
}
