//! Edge Optimizer — Sprint 82: Tactical Pivot & Distributed SAE Audit MVP
//!
//! Dynamic model selection based on available system RAM, WASM async pipeline
//! optimization, and fallback strategies for edge devices.
//!
//! ### Feature Gates
//! | Feature | Módulo | Descripción |
//! |---|---|---|
//! | `v9.18-mvp-deployment` | edge_optimizer | EdgeOptimizer — RAM-aware model selection, WASM async pipeline |

use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Errors
// ============================================================================

/// Errors produced by edge optimization operations.
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeError {
    /// Insufficient RAM for any available model.
    InsufficientRam { available_mb: u32, minimum_mb: u32 },
    /// Requested model not available in the catalog.
    ModelNotFound(String),
    /// WASM pipeline failed to initialize.
    WasmInitFailed(String),
    /// Invalid model configuration.
    InvalidConfig(String),
}

impl fmt::Display for EdgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EdgeError::InsufficientRam {
                available_mb,
                minimum_mb,
            } => {
                write!(
                    f,
                    "Insufficient RAM: {} MB available, {} MB minimum required",
                    available_mb, minimum_mb
                )
            }
            EdgeError::ModelNotFound(name) => write!(f, "Model not found: {}", name),
            EdgeError::WasmInitFailed(msg) => write!(f, "WASM init failed: {}", msg),
            EdgeError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
        }
    }
}

// ============================================================================
// Model Catalog
// ============================================================================

/// Supported models with their resource requirements.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelSpec {
    /// Model identifier (e.g., "qwen3.5:2b").
    pub name: String,
    /// Minimum RAM required in MB.
    pub min_ram_mb: u32,
    /// Recommended RAM in MB for optimal performance.
    pub recommended_ram_mb: u32,
    /// Estimated boot latency in milliseconds.
    pub boot_latency_ms: u64,
    /// Whether this model supports WASM async inference.
    pub wasm_async: bool,
}

impl ModelSpec {
    pub fn new(
        name: String,
        min_ram_mb: u32,
        recommended_ram_mb: u32,
        boot_latency_ms: u64,
        wasm_async: bool,
    ) -> Self {
        Self {
            name,
            min_ram_mb,
            recommended_ram_mb,
            boot_latency_ms,
            wasm_async,
        }
    }
}

impl fmt::Display for ModelSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (RAM: {}-{} MB, boot: {}ms, wasm: {})",
            self.name,
            self.min_ram_mb,
            self.recommended_ram_mb,
            self.boot_latency_ms,
            self.wasm_async
        )
    }
}

/// Default model catalog for edge deployment.
pub fn default_catalog() -> Vec<ModelSpec> {
    vec![
        ModelSpec::new("qwen3.5:2b".to_string(), 2048, 4096, 300, true),
        ModelSpec::new("qwen3.5:4b".to_string(), 4096, 8192, 500, true),
        ModelSpec::new("qwen3.5:8b".to_string(), 8192, 16384, 800, false),
        ModelSpec::new("micro-sae".to_string(), 512, 1024, 100, true),
    ]
}

// ============================================================================
// Optimization Config
// ============================================================================

/// Configuration for the edge optimizer.
#[derive(Debug, Clone)]
pub struct EdgeConfig {
    /// Default model to use when RAM is sufficient.
    pub default_model: String,
    /// Fallback model when default cannot run.
    pub fallback_model: String,
    /// Minimum RAM threshold in MB below which only micro-sae runs.
    pub micro_threshold_mb: u32,
    /// Enable WASM async pipeline.
    pub wasm_async_enabled: bool,
    /// Maximum concurrent inference workers.
    pub max_workers: usize,
}

impl EdgeConfig {
    pub fn default_stuartian() -> Self {
        Self {
            default_model: "qwen3.5:2b".to_string(),
            fallback_model: "micro-sae".to_string(),
            micro_threshold_mb: 1024,
            wasm_async_enabled: true,
            max_workers: 4,
        }
    }

    pub fn validate(&self) -> Result<(), EdgeError> {
        if self.default_model.is_empty() {
            return Err(EdgeError::InvalidConfig(
                "default_model cannot be empty".to_string(),
            ));
        }
        if self.fallback_model.is_empty() {
            return Err(EdgeError::InvalidConfig(
                "fallback_model cannot be empty".to_string(),
            ));
        }
        if self.micro_threshold_mb < 256 {
            return Err(EdgeError::InvalidConfig(
                "micro_threshold_mb must be >= 256".to_string(),
            ));
        }
        if self.max_workers == 0 {
            return Err(EdgeError::InvalidConfig(
                "max_workers must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for EdgeConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

// ============================================================================
// Pipeline State
// ============================================================================

/// State of the WASM async inference pipeline.
#[derive(Debug, Clone, PartialEq)]
pub enum PipelineState {
    /// Pipeline is idle and ready.
    Idle,
    /// Pipeline is actively processing.
    Processing { active_workers: usize },
    /// Pipeline encountered an error.
    Error(String),
}

impl fmt::Display for PipelineState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PipelineState::Idle => write!(f, "Idle"),
            PipelineState::Processing { active_workers } => {
                write!(f, "Processing ({} workers)", active_workers)
            }
            PipelineState::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

// ============================================================================
// Optimization Record
// ============================================================================

/// Record of a model selection decision.
#[derive(Debug, Clone)]
pub struct OptimizationRecord {
    pub requested_model: String,
    pub selected_model: String,
    pub available_ram_mb: u32,
    pub timestamp_ms: u64,
    pub fallback_triggered: bool,
}

impl fmt::Display for OptimizationRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fallback = if self.fallback_triggered { "YES" } else { "NO" };
        write!(
            f,
            "Optimization: {} -> {} (RAM: {} MB, fallback: {}, ts: {})",
            self.requested_model,
            self.selected_model,
            self.available_ram_mb,
            fallback,
            self.timestamp_ms
        )
    }
}

// ============================================================================
// EdgeOptimizer
// ============================================================================

/// Core edge optimizer that selects models based on available resources.
pub struct EdgeOptimizer {
    config: EdgeConfig,
    catalog: Vec<ModelSpec>,
    pipeline_state: PipelineState,
    records: Vec<OptimizationRecord>,
}

impl EdgeOptimizer {
    pub fn new() -> Self {
        Self {
            config: EdgeConfig::default_stuartian(),
            catalog: default_catalog(),
            pipeline_state: PipelineState::Idle,
            records: Vec::new(),
        }
    }

    pub fn with_config(config: EdgeConfig) -> Result<Self, EdgeError> {
        config.validate()?;
        Ok(Self {
            config,
            catalog: default_catalog(),
            pipeline_state: PipelineState::Idle,
            records: Vec::new(),
        })
    }

    /// Select the optimal model based on available RAM.
    /// If the preferred model fits, use it. Otherwise, fall back to the largest model that fits.
    pub fn select_model(
        &mut self,
        preferred: &str,
        available_ram_mb: u32,
        timestamp_ms: u64,
    ) -> Result<String, EdgeError> {
        // Check if preferred model fits
        if let Some(spec) = self.catalog.iter().find(|s| s.name == preferred) {
            if available_ram_mb >= spec.min_ram_mb {
                self.record_selection(
                    preferred.to_string(),
                    preferred.to_string(),
                    available_ram_mb,
                    timestamp_ms,
                    false,
                );
                return Ok(preferred.to_string());
            }
        }

        // Find the largest model that fits
        let mut candidates: Vec<&ModelSpec> = self
            .catalog
            .iter()
            .filter(|s| available_ram_mb >= s.min_ram_mb)
            .collect();

        candidates.sort_by(|a, b| b.min_ram_mb.cmp(&a.min_ram_mb));

        if let Some(best) = candidates.first() {
            let selected = best.name.clone();
            let fallback = selected != preferred;
            self.record_selection(
                preferred.to_string(),
                selected.clone(),
                available_ram_mb,
                timestamp_ms,
                fallback,
            );
            return Ok(selected);
        }

        // Check micro threshold
        if available_ram_mb < self.config.micro_threshold_mb {
            return Err(EdgeError::InsufficientRam {
                available_mb: available_ram_mb,
                minimum_mb: self.config.micro_threshold_mb,
            });
        }

        Err(EdgeError::ModelNotFound(preferred.to_string()))
    }

    /// Activate the WASM async pipeline.
    pub fn activate_pipeline(&mut self) -> Result<(), EdgeError> {
        if !self.config.wasm_async_enabled {
            return Err(EdgeError::WasmInitFailed(
                "WASM async disabled in config".to_string(),
            ));
        }
        self.pipeline_state = PipelineState::Processing { active_workers: 0 };
        Ok(())
    }

    /// Simulate starting a worker in the pipeline.
    pub fn start_worker(&mut self) -> Result<usize, EdgeError> {
        match &self.pipeline_state {
            PipelineState::Processing { active_workers } => {
                let new_count = active_workers + 1;
                if new_count > self.config.max_workers {
                    return Err(EdgeError::InvalidConfig(format!(
                        "Max workers ({}) reached",
                        self.config.max_workers
                    )));
                }
                self.pipeline_state = PipelineState::Processing {
                    active_workers: new_count,
                };
                Ok(new_count)
            }
            PipelineState::Idle => Err(EdgeError::WasmInitFailed(
                "Pipeline not activated".to_string(),
            )),
            PipelineState::Error(msg) => Err(EdgeError::WasmInitFailed(msg.clone())),
        }
    }

    /// Simulate stopping a worker.
    pub fn stop_worker(&mut self) {
        if let PipelineState::Processing { active_workers } = &self.pipeline_state {
            let new_count = active_workers.saturating_sub(1);
            if new_count == 0 {
                self.pipeline_state = PipelineState::Idle;
            } else {
                self.pipeline_state = PipelineState::Processing {
                    active_workers: new_count,
                };
            }
        }
    }

    /// Get the model spec for a given name.
    pub fn get_model_spec(&self, name: &str) -> Option<&ModelSpec> {
        self.catalog.iter().find(|s| s.name == name)
    }

    /// Get all models that fit in the given RAM.
    pub fn compatible_models(&self, available_ram_mb: u32) -> Vec<&ModelSpec> {
        self.catalog
            .iter()
            .filter(|s| available_ram_mb >= s.min_ram_mb)
            .collect()
    }

    /// Get optimization records.
    pub fn records(&self) -> &[OptimizationRecord] {
        &self.records
    }

    /// Get current pipeline state.
    pub fn pipeline_state(&self) -> &PipelineState {
        &self.pipeline_state
    }

    /// Reset optimizer state.
    pub fn reset(&mut self) {
        self.pipeline_state = PipelineState::Idle;
        self.records.clear();
    }

    fn record_selection(
        &mut self,
        requested: String,
        selected: String,
        ram: u32,
        ts: u64,
        fallback: bool,
    ) {
        self.records.push(OptimizationRecord {
            requested_model: requested,
            selected_model: selected,
            available_ram_mb: ram,
            timestamp_ms: ts,
            fallback_triggered: fallback,
        });
    }
}

impl Default for EdgeOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EdgeOptimizer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EdgeOptimizer(default={}, pipeline={}, records={})",
            self.config.default_model,
            self.pipeline_state,
            self.records.len()
        )
    }
}

// ============================================================================
// Standalone Functions
// ============================================================================

/// Select the optimal model based on available RAM.
/// This is a standalone function for use without an EdgeOptimizer instance.
pub fn select_optimal_model(preferred: &str, available_ram_mb: u32) -> String {
    let catalog = default_catalog();

    // Check if preferred model fits
    if let Some(spec) = catalog.iter().find(|s| s.name == preferred) {
        if available_ram_mb >= spec.min_ram_mb {
            return preferred.to_string();
        }
    }

    // Find largest model that fits
    let mut candidates: Vec<&ModelSpec> = catalog
        .iter()
        .filter(|s| available_ram_mb >= s.min_ram_mb)
        .collect();

    candidates.sort_by(|a, b| b.min_ram_mb.cmp(&a.min_ram_mb));

    if let Some(best) = candidates.first() {
        return best.name.clone();
    }

    // Ultimate fallback
    "micro-sae".to_string()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = EdgeConfig::default_stuartian();
        assert_eq!(config.default_model, "qwen3.5:2b");
        assert_eq!(config.fallback_model, "micro-sae");
        assert!(config.wasm_async_enabled);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = EdgeConfig::default_stuartian();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_empty_default() {
        let mut config = EdgeConfig::default_stuartian();
        config.default_model = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_workers() {
        let mut config = EdgeConfig::default_stuartian();
        config.max_workers = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_low_threshold() {
        let mut config = EdgeConfig::default_stuartian();
        config.micro_threshold_mb = 128;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_default_catalog() {
        let catalog = default_catalog();
        assert_eq!(catalog.len(), 4);
        assert_eq!(catalog[0].name, "qwen3.5:2b");
    }

    #[test]
    fn test_model_spec_display() {
        let spec = ModelSpec::new("test".to_string(), 1024, 2048, 200, true);
        let display = format!("{}", spec);
        assert!(display.contains("test"));
        assert!(display.contains("1024"));
    }

    #[test]
    fn test_engine_creation() {
        let engine = EdgeOptimizer::new();
        assert_eq!(engine.pipeline_state(), &PipelineState::Idle);
        assert!(engine.records().is_empty());
    }

    #[test]
    fn test_engine_with_config() {
        let config = EdgeConfig::default_stuartian();
        let engine = EdgeOptimizer::with_config(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_select_model_fits() {
        let mut engine = EdgeOptimizer::new();
        let result = engine.select_model("qwen3.5:2b", 4096, 1000);
        assert_eq!(result.unwrap(), "qwen3.5:2b");
    }

    #[test]
    fn test_select_model_fallback_to_smaller() {
        let mut engine = EdgeOptimizer::new();
        let result = engine.select_model("qwen3.5:8b", 4096, 1000);
        // 8b needs 8192, so fallback to 4b (needs 4096)
        assert_eq!(result.unwrap(), "qwen3.5:4b");
    }

    #[test]
    fn test_select_model_fallback_to_micro() {
        let mut engine = EdgeOptimizer::new();
        let result = engine.select_model("qwen3.5:2b", 768, 1000);
        // 2b needs 2048, micro needs 512
        assert_eq!(result.unwrap(), "micro-sae");
    }

    #[test]
    fn test_select_model_insufficient_ram() {
        let mut engine = EdgeOptimizer::new();
        let result = engine.select_model("qwen3.5:2b", 256, 1000);
        // 256 MB < micro_threshold (1024)
        assert!(result.is_err());
    }

    #[test]
    fn test_select_model_fallback_recorded() {
        let mut engine = EdgeOptimizer::new();
        let _ = engine.select_model("qwen3.5:8b", 4096, 1000);
        let records = engine.records();
        assert_eq!(records.len(), 1);
        assert!(records[0].fallback_triggered);
    }

    #[test]
    fn test_select_model_no_fallback() {
        let mut engine = EdgeOptimizer::new();
        let _ = engine.select_model("qwen3.5:2b", 8192, 1000);
        let records = engine.records();
        assert_eq!(records.len(), 1);
        assert!(!records[0].fallback_triggered);
    }

    #[test]
    fn test_activate_pipeline() {
        let mut engine = EdgeOptimizer::new();
        assert!(engine.activate_pipeline().is_ok());
        matches!(engine.pipeline_state(), PipelineState::Processing { .. });
    }

    #[test]
    fn test_activate_pipeline_disabled() {
        let mut config = EdgeConfig::default_stuartian();
        config.wasm_async_enabled = false;
        let mut engine = EdgeOptimizer::with_config(config).unwrap();
        assert!(engine.activate_pipeline().is_err());
    }

    #[test]
    fn test_start_worker() {
        let mut engine = EdgeOptimizer::new();
        engine.activate_pipeline().unwrap();
        let workers = engine.start_worker().unwrap();
        assert_eq!(workers, 1);
    }

    #[test]
    fn test_start_worker_max_reached() {
        let mut config = EdgeConfig::default_stuartian();
        config.max_workers = 1;
        let mut engine = EdgeOptimizer::with_config(config).unwrap();
        engine.activate_pipeline().unwrap();
        engine.start_worker().unwrap();
        assert!(engine.start_worker().is_err());
    }

    #[test]
    fn test_stop_worker_returns_to_idle() {
        let mut engine = EdgeOptimizer::new();
        engine.activate_pipeline().unwrap();
        engine.start_worker().unwrap();
        engine.stop_worker();
        assert_eq!(engine.pipeline_state(), &PipelineState::Idle);
    }

    #[test]
    fn test_compatible_models() {
        let engine = EdgeOptimizer::new();
        let models = engine.compatible_models(4096);
        assert_eq!(models.len(), 3); // 2b, 4b, micro
    }

    #[test]
    fn test_get_model_spec() {
        let engine = EdgeOptimizer::new();
        let spec = engine.get_model_spec("qwen3.5:2b");
        assert!(spec.is_some());
        assert_eq!(spec.unwrap().wasm_async, true);
    }

    #[test]
    fn test_get_model_spec_not_found() {
        let engine = EdgeOptimizer::new();
        let spec = engine.get_model_spec("nonexistent");
        assert!(spec.is_none());
    }

    #[test]
    fn test_reset() {
        let mut engine = EdgeOptimizer::new();
        engine.activate_pipeline().unwrap();
        let _ = engine.select_model("qwen3.5:2b", 4096, 1000);
        engine.reset();
        assert_eq!(engine.pipeline_state(), &PipelineState::Idle);
        assert!(engine.records().is_empty());
    }

    #[test]
    fn test_display() {
        let engine = EdgeOptimizer::new();
        let display = format!("{}", engine);
        assert!(display.contains("EdgeOptimizer"));
    }

    #[test]
    fn test_record_display() {
        let record = OptimizationRecord {
            requested_model: "qwen3.5:8b".to_string(),
            selected_model: "qwen3.5:2b".to_string(),
            available_ram_mb: 3000,
            timestamp_ms: 1000,
            fallback_triggered: true,
        };
        let display = format!("{}", record);
        assert!(display.contains("qwen3.5:8b"));
        assert!(display.contains("YES"));
    }

    #[test]
    fn test_pipeline_state_display() {
        assert_eq!(format!("{}", PipelineState::Idle), "Idle");
        let processing = PipelineState::Processing { active_workers: 2 };
        assert!(format!("{}", processing).contains("2"));
        let error = PipelineState::Error("test".to_string());
        assert!(format!("{}", error).contains("test"));
    }

    #[test]
    fn test_error_display() {
        let err = EdgeError::InsufficientRam {
            available_mb: 512,
            minimum_mb: 2048,
        };
        assert!(format!("{}", err).contains("512"));
    }

    #[test]
    fn test_standalone_select_optimal() {
        let model = select_optimal_model("qwen3.5:2b", 4096);
        assert_eq!(model, "qwen3.5:2b");
    }

    #[test]
    fn test_standalone_select_fallback() {
        let model = select_optimal_model("qwen3.5:8b", 3000);
        assert_eq!(model, "qwen3.5:2b");
    }

    #[test]
    fn test_standalone_select_micro() {
        let model = select_optimal_model("qwen3.5:2b", 600);
        assert_eq!(model, "micro-sae");
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = EdgeOptimizer::new();

        // Select model with sufficient RAM
        let model = engine.select_model("qwen3.5:2b", 8192, 1000).unwrap();
        assert_eq!(model, "qwen3.5:2b");

        // Select model with limited RAM triggers fallback
        let model = engine.select_model("qwen3.5:8b", 4096, 2000).unwrap();
        assert_eq!(model, "qwen3.5:4b");

        // Activate pipeline
        engine.activate_pipeline().unwrap();
        let workers = engine.start_worker().unwrap();
        assert_eq!(workers, 1);

        // Stop worker
        engine.stop_worker();
        assert_eq!(engine.pipeline_state(), &PipelineState::Idle);

        // Verify records
        assert_eq!(engine.records().len(), 2);
        assert!(!engine.records()[0].fallback_triggered);
        assert!(engine.records()[1].fallback_triggered);

        // Reset
        engine.reset();
        assert!(engine.records().is_empty());
    }
}
