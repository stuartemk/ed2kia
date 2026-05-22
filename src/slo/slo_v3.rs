//! SLO/SLA v3 — Service Level Objective/Agreement con evaluación predictiva
//!
//! Extiende el motor SLO v1 con contratos predictivos basados en ventanas
//! deslizantes. Soporta evaluación estática y predictiva con fallback
//! automático cuando `prediction_confidence < 0.75`.
//!
//! Feature-gated: `#[cfg(feature = "v1.2-sprint3")]`

#[cfg(feature = "v1.2-sprint3")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.2-sprint3")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "v1.2-sprint3")]
use std::time::{Instant, SystemTime, UNIX_EPOCH};
#[cfg(feature = "v1.2-sprint3")]
use thiserror::Error;
#[cfg(feature = "v1.2-sprint3")]
use tracing::{debug, info};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[cfg(feature = "v1.2-sprint3")]
#[derive(Debug, Error)]
pub enum SLOv3Error {
    #[error("SLO not found: {0}")]
    SloNotFound(String),

    #[error("Invalid prediction confidence: {0}")]
    InvalidConfidence(f64),

    #[error("Window size exceeded: {0}")]
    WindowExceeded(usize),

    #[error("Metric series too short for prediction")]
    InsufficientData,

    #[error("Contract evaluation failed: {0}")]
    ContractError(String),
}

// ---------------------------------------------------------------------------
// SLO Status
// ---------------------------------------------------------------------------

/// Estado de cumplimiento de un SLO.
#[cfg(feature = "v1.2-sprint3")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SLOv3Status {
    Compliant,
    Warning,
    Critical,
    PredictiveBreach,
}

#[cfg(feature = "v1.2-sprint3")]
impl std::fmt::Display for SLOv3Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SLOv3Status::Compliant => write!(f, "Compliant"),
            SLOv3Status::Warning => write!(f, "Warning"),
            SLOv3Status::Critical => write!(f, "Critical"),
            SLOv3Status::PredictiveBreach => write!(f, "PredictiveBreach"),
        }
    }
}

// ---------------------------------------------------------------------------
// SLO v3 Config
// ---------------------------------------------------------------------------

/// Configuración de un SLO v3 con evaluación predictiva.
#[cfg(feature = "v1.2-sprint3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLOv3Config {
    /// Nombre del SLO.
    pub name: String,
    /// Métrica asociada.
    pub metric_key: String,
    /// Valor objetivo.
    pub target: f64,
    /// Umbral de advertencia (fracción del target).
    pub warning_threshold: f64,
    /// Ventana deslizante en segundos.
    pub window_seconds: u64,
    /// Puntos mínimos para predicción.
    pub min_prediction_points: usize,
    /// Confianza mínima para usar predicción.
    pub min_prediction_confidence: f64,
    /// Máximo de ventanas consecutivas en breach antes de degradación.
    pub max_breach_windows: usize,
}

#[cfg(feature = "v1.2-sprint3")]
impl Default for SLOv3Config {
    fn default() -> Self {
        Self {
            name: Default::default(),
            metric_key: Default::default(),
            target: 99.9,
            warning_threshold: 0.95,
            window_seconds: 30,
            min_prediction_points: 10,
            min_prediction_confidence: 0.75,
            max_breach_windows: 3,
        }
    }
}

// ---------------------------------------------------------------------------
// Metric Point
// ---------------------------------------------------------------------------

/// Punto de métrica con timestamp.
#[cfg(feature = "v1.2-sprint3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub value: f64,
    pub timestamp: u64,
}

#[cfg(feature = "v1.2-sprint3")]
impl MetricPoint {
    pub fn new(value: f64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self { value, timestamp }
    }
}

// ---------------------------------------------------------------------------
// Prediction Result
// ---------------------------------------------------------------------------

/// Resultado de evaluación predictiva.
#[cfg(feature = "v1.2-sprint3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResult {
    /// Valor predicho para el siguiente intervalo.
    pub predicted_value: f64,
    /// Confianza de la predicción [0.0, 1.0].
    pub confidence: f64,
    /// Tendencia: positivo = mejorando, negativo = degradando.
    pub trend: f64,
    /// Se usó modo predictivo o estático.
    pub used_prediction: bool,
}

// ---------------------------------------------------------------------------
// SLO v3 Evaluation Result
// ---------------------------------------------------------------------------

/// Resultado de evaluación de un SLO v3.
#[cfg(feature = "v1.2-sprint3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLOv3Result {
    pub status: SLOv3Status,
    pub current_value: f64,
    pub target: f64,
    pub breach_count: usize,
    pub prediction: Option<PredictionResult>,
    pub action: String,
}

// ---------------------------------------------------------------------------
// SLO v3 Entry
// ---------------------------------------------------------------------------

/// Entrada de SLO v3 con serie temporal.
#[cfg(feature = "v1.2-sprint3")]
#[derive(Debug, Clone)]
pub struct SLOv3Entry {
    pub config: SLOv3Config,
    pub series: VecDeque<MetricPoint>,
    pub breach_count: usize,
    pub last_evaluation: Option<Instant>,
    pub consecutive_breaches: usize,
}

// ---------------------------------------------------------------------------
// SLO v3 Engine
// ---------------------------------------------------------------------------

/// Motor de evaluación SLO v3 con predicción.
#[cfg(feature = "v1.2-sprint3")]
pub struct SLOv3Engine {
    slo_entries: HashMap<String, SLOv3Entry>,
}

#[cfg(feature = "v1.2-sprint3")]
impl SLOv3Engine {
    /// Crea un nuevo motor SLO v3.
    pub fn new() -> Self {
        Self {
            slo_entries: HashMap::new(),
        }
    }

    /// Registra un SLO con configuración.
    pub fn register_slo(&mut self, config: SLOv3Config) {
        let entry = SLOv3Entry {
            series: VecDeque::with_capacity(config.min_prediction_points.max(60)),
            breach_count: 0,
            last_evaluation: None,
            consecutive_breaches: 0,
            config: config.clone(),
        };
        self.slo_entries.insert(config.name.clone(), entry);
        info!(slo = %config.name, "SLO v3 registered");
    }

    /// Remueve un SLO registrado.
    pub fn remove_slo(&mut self, name: &str) -> Result<(), SLOv3Error> {
        self.slo_entries
            .remove(name)
            .map(|_| ())
            .ok_or(SLOv3Error::SloNotFound(name.to_string()))
    }

    /// Registra un punto de métrica para un SLO.
    pub fn record_metric(&mut self, slo_name: &str, value: f64) -> Result<(), SLOv3Error> {
        let entry = self
            .slo_entries
            .get_mut(slo_name)
            .ok_or(SLOv3Error::SloNotFound(slo_name.to_string()))?;

        let point = MetricPoint::new(value);
        let cutoff_ts = point.timestamp.saturating_sub(entry.config.window_seconds);
        entry.series.push_back(point);

        // Retener puntos dentro de la ventana
        while let Some(front) = entry.series.front() {
            if front.timestamp < cutoff_ts {
                entry.series.pop_front();
            } else {
                break;
            }
        }
        debug!(slo = %slo_name, value, "metric recorded");
        Ok(())
    }

    /// Evalúa todos los SLO registrados.
    pub fn evaluate_all(&mut self) -> HashMap<String, SLOv3Result> {
        let mut results = HashMap::new();
        let names: Vec<String> = self.slo_entries.keys().cloned().collect();
        for name in names {
            if let Ok(result) = self.evaluate(&name) {
                results.insert(name, result);
            }
        }
        results
    }

    /// Evalúa un SLO específico.
    pub fn evaluate(&mut self, slo_name: &str) -> Result<SLOv3Result, SLOv3Error> {
        let entry = self
            .slo_entries
            .get_mut(slo_name)
            .ok_or(SLOv3Error::SloNotFound(slo_name.to_string()))?;

        entry.last_evaluation = Some(Instant::now());

        if entry.series.is_empty() {
            return Ok(SLOv3Result {
                status: SLOv3Status::Warning,
                current_value: 0.0,
                target: entry.config.target,
                breach_count: entry.breach_count,
                prediction: None,
                action: "no_data".into(),
            });
        }

        let current = entry.series.back().unwrap().value;
        let target = entry.config.target;
        let warning = target * entry.config.warning_threshold;

        // Intentar predicción (clonar datos necesarios para evitar borrow conflict)
        let series_clone: Vec<f64> = entry.series.iter().map(|p| p.value).collect();
        let config_clone = entry.config.clone();
        let prediction = if series_clone.len() >= config_clone.min_prediction_points {
            Self::predict_from_data(&series_clone, &config_clone)
        } else {
            None
        };

        // Determinar estado
        let (status, action) = if current >= target {
            entry.consecutive_breaches = 0;
            (SLOv3Status::Compliant, "none")
        } else if current >= warning {
            (SLOv3Status::Warning, "alert_sent")
        } else {
            entry.consecutive_breaches += 1;
            entry.breach_count += 1;
            if entry.consecutive_breaches >= entry.config.max_breach_windows {
                (SLOv3Status::Critical, "degradation_triggered")
            } else {
                (SLOv3Status::Warning, "alert_sent")
            }
        };

        // Verificar breach predictivo
        let final_status = if let Some(pred) = &prediction {
            if pred.used_prediction && pred.predicted_value < warning {
                SLOv3Status::PredictiveBreach
            } else {
                status
            }
        } else {
            status
        };

        let final_action = if final_status == SLOv3Status::PredictiveBreach {
            "predictive_alert"
        } else {
            action
        };

        Ok(SLOv3Result {
            status: final_status,
            current_value: current,
            target,
            breach_count: entry.breach_count,
            prediction,
            action: final_action.to_string(),
        })
    }

    /// Genera predicción basada en datos crudos (para evitar borrow conflicts).
    fn predict_from_data(values: &[f64], config: &SLOv3Config) -> Option<PredictionResult> {
        let n = values.len();
        if n < config.min_prediction_points {
            return None;
        }
        do_linear_regression(values, n, config)
    }

    /// Genera predicción basada en serie temporal.
    fn predict(&self, entry: &SLOv3Entry) -> Option<PredictionResult> {
        let values: Vec<f64> = entry.series.iter().map(|p| p.value).collect();
        Self::predict_from_data(&values, &entry.config)
    }

    /// Obtiene el resultado de evaluación más reciente.
    pub fn get_slo_status(&self, slo_name: &str) -> Option<&SLOv3Entry> {
        self.slo_entries.get(slo_name)
    }

    /// Retorna el número de SLOs registrados.
    pub fn slo_count(&self) -> usize {
        self.slo_entries.len()
    }
}

#[cfg(feature = "v1.2-sprint3")]
impl Default for SLOv3Engine {
    fn default() -> Self {
        Self::new()
    }
}

/// Función auxiliar de regresión lineal compartida.
fn do_linear_regression(
    values: &[f64],
    n: usize,
    config: &SLOv3Config,
) -> Option<PredictionResult> {
    let sum_x: f64 = (0..n).map(|i| i as f64).sum();
    let sum_y: f64 = values.iter().sum();
    let sum_xy: f64 = (0..n).map(|i| i as f64 * values[i]).sum();
    let sum_x2: f64 = (0..n)
        .map(|i| {
            let x = i as f64;
            x * x
        })
        .sum();

    let denominator = (n as f64) * sum_x2 - sum_x * sum_x;
    if denominator.abs() < 1e-10 {
        return None;
    }

    let b = ((n as f64) * sum_xy - sum_x * sum_y) / denominator;
    let a = (sum_y - b * sum_x) / (n as f64);
    let predicted = a + b * (n as f64);

    let mean_y = sum_y / (n as f64);
    let ss_tot: f64 = values.iter().map(|y| (y - mean_y).powi(2)).sum();
    let ss_res: f64 = values
        .iter()
        .enumerate()
        .map(|(i, y)| {
            let pred = a + b * (i as f64);
            (y - pred).powi(2)
        })
        .sum();

    let r_squared = if ss_tot.abs() < 1e-10 {
        0.0
    } else {
        (1.0 - ss_res / ss_tot).max(0.0)
    };

    let confidence = r_squared.min(1.0);
    let use_prediction = confidence >= config.min_prediction_confidence;

    Some(PredictionResult {
        predicted_value: predicted,
        confidence,
        trend: b,
        used_prediction: use_prediction,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_config(name: &str) -> SLOv3Config {
        SLOv3Config {
            name: name.to_string(),
            metric_key: "latency".into(),
            target: 100.0,
            warning_threshold: 0.8,
            window_seconds: 30,
            min_prediction_points: 5,
            min_prediction_confidence: 0.75,
            max_breach_windows: 3,
        }
    }

    #[test]
    fn test_engine_creation() {
        let engine = SLOv3Engine::new();
        assert_eq!(engine.slo_count(), 0);
    }

    #[test]
    fn test_register_slo() {
        let mut engine = SLOv3Engine::new();
        engine.register_slo(make_config("test"));
        assert_eq!(engine.slo_count(), 1);
    }

    #[test]
    fn test_remove_slo() {
        let mut engine = SLOv3Engine::new();
        engine.register_slo(make_config("test"));
        assert!(engine.remove_slo("test").is_ok());
        assert_eq!(engine.slo_count(), 0);
    }

    #[test]
    fn test_remove_missing_slo() {
        let mut engine = SLOv3Engine::new();
        assert!(engine.remove_slo("missing").is_err());
    }

    #[test]
    fn test_record_metric() {
        let mut engine = SLOv3Engine::new();
        engine.register_slo(make_config("test"));
        assert!(engine.record_metric("test", 95.0).is_ok());
    }

    #[test]
    fn test_evaluate_compliant() {
        let mut engine = SLOv3Engine::new();
        engine.register_slo(make_config("test"));
        for _ in 0..10 {
            engine.record_metric("test", 100.0).unwrap();
        }
        let result = engine.evaluate("test").unwrap();
        assert_eq!(result.status, SLOv3Status::Compliant);
    }

    #[test]
    fn test_evaluate_warning() {
        let mut engine = SLOv3Engine::new();
        engine.register_slo(make_config("test"));
        for _ in 0..10 {
            engine.record_metric("test", 85.0).unwrap();
        }
        let result = engine.evaluate("test").unwrap();
        assert_eq!(result.status, SLOv3Status::Warning);
    }

    #[test]
    fn test_evaluate_critical() {
        let mut engine = SLOv3Engine::new();
        engine.register_slo(make_config("test"));
        for _ in 0..10 {
            engine.record_metric("test", 50.0).unwrap();
        }
        // Evaluate multiple times to trigger consecutive breaches
        for _ in 0..5 {
            engine.record_metric("test", 50.0).unwrap();
            engine.evaluate("test").unwrap();
        }
        let result = engine.evaluate("test").unwrap();
        assert_eq!(result.status, SLOv3Status::Critical);
    }

    #[test]
    fn test_prediction_with_trend() {
        let mut engine = SLOv3Engine::new();
        engine.register_slo(make_config("test"));
        // Descending trend
        for i in 0..10 {
            engine.record_metric("test", (100 - i) as f64).unwrap();
        }
        let result = engine.evaluate("test").unwrap();
        assert!(result.prediction.is_some());
        let pred = result.prediction.unwrap();
        assert!(pred.trend < 0.0);
    }

    #[test]
    fn test_prediction_confidence() {
        let mut engine = SLOv3Engine::new();
        engine.register_slo(make_config("test"));
        // Perfect linear trend
        for i in 0..10 {
            engine.record_metric("test", i as f64 * 10.0).unwrap();
        }
        let result = engine.evaluate("test").unwrap();
        let pred = result.prediction.unwrap();
        assert!(pred.confidence > 0.9);
    }

    #[test]
    fn test_evaluate_no_data() {
        let mut engine = SLOv3Engine::new();
        engine.register_slo(make_config("test"));
        let result = engine.evaluate("test").unwrap();
        assert_eq!(result.status, SLOv3Status::Warning);
        assert_eq!(result.action, "no_data");
    }

    #[test]
    fn test_evaluate_all() {
        let mut engine = SLOv3Engine::new();
        engine.register_slo(make_config("a"));
        engine.register_slo(make_config("b"));
        engine.record_metric("a", 100.0).unwrap();
        engine.record_metric("b", 50.0).unwrap();
        let results = engine.evaluate_all();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_predictive_breach() {
        let mut engine = SLOv3Engine::new();
        let config = SLOv3Config {
            min_prediction_confidence: 0.5,
            ..make_config("test")
        };
        engine.register_slo(config);
        // Strong descending trend
        for i in 0..15 {
            engine
                .record_metric("test", (100.0 - i as f64 * 3.0))
                .unwrap();
        }
        let result = engine.evaluate("test").unwrap();
        assert_eq!(result.status, SLOv3Status::PredictiveBreach);
    }

    #[test]
    fn test_status_display() {
        assert_eq!(format!("{}", SLOv3Status::Compliant), "Compliant");
        assert_eq!(format!("{}", SLOv3Status::Warning), "Warning");
        assert_eq!(format!("{}", SLOv3Status::Critical), "Critical");
        assert_eq!(
            format!("{}", SLOv3Status::PredictiveBreach),
            "PredictiveBreach"
        );
    }

    #[test]
    fn test_error_display() {
        let err = SLOv3Error::SloNotFound("x".into());
        assert!(format!("{}", err).contains("x"));
    }

    #[test]
    fn test_config_default() {
        let config = SLOv3Config::default();
        assert_eq!(config.target, 99.9);
        assert_eq!(config.window_seconds, 30);
    }

    #[test]
    fn test_engine_default() {
        let engine = SLOv3Engine::default();
        assert_eq!(engine.slo_count(), 0);
    }

    #[test]
    fn test_metric_point_creation() {
        let point = MetricPoint::new(42.0);
        assert_eq!(point.value, 42.0);
        assert!(point.timestamp > 0);
    }

    #[test]
    fn test_window_pruning() {
        let mut engine = SLOv3Engine::new();
        let config = SLOv3Config {
            window_seconds: 1,
            ..make_config("test")
        };
        engine.register_slo(config);
        // Registrar 5 metricas rapidamente
        for _ in 0..5 {
            engine.record_metric("test", 100.0).unwrap();
        }
        // Esperar suficiente para que la ventana expire
        std::thread::sleep(Duration::from_secs(2));
        // Registrar una nueva metrica fuera de ventana
        engine.record_metric("test", 100.0).unwrap();
        let entry = engine.get_slo_status("test").unwrap();
        // Las metricas antiguas deben haber sido eliminadas
        assert!(entry.series.len() < 6);
    }

    #[test]
    fn test_get_slo_status() {
        let mut engine = SLOv3Engine::new();
        engine.register_slo(make_config("test"));
        assert!(engine.get_slo_status("test").is_some());
        assert!(engine.get_slo_status("missing").is_none());
    }
}
