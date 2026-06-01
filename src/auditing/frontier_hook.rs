//! Frontier Model Activation Hooking — Sprint 70: Civilization-Scale Architecture
//!
//! Intercepts transformer layer activations (attention, MLP, RMSNorm)
//! for real-time SAE feature extraction and ethical analysis.

/// Error types for activation hooking.
#[derive(Debug, Clone, PartialEq)]
pub enum HookError {
    /// Layer not found in model.
    LayerNotFound { layer: String },
    /// Tensor shape mismatch.
    ShapeMismatch { expected: usize, actual: usize },
    /// Hook limit exceeded.
    HookLimitExceeded,
}

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HookError::LayerNotFound { layer } => write!(f, "layer not found: {}", layer),
            HookError::ShapeMismatch { expected, actual } => {
                write!(
                    f,
                    "shape mismatch: expected={}, actual={}",
                    expected, actual
                )
            }
            HookError::HookLimitExceeded => write!(f, "hook limit exceeded"),
        }
    }
}

impl std::error::Error for HookError {}

/// Configuration for activation hooks.
#[derive(Debug, Clone)]
pub struct HookConfig {
    /// Maximum hooks per model.
    pub max_hooks: usize,
    /// Activation threshold for SAE processing.
    pub activation_threshold: f64,
    /// Enable attention layer hooks.
    pub hook_attention: bool,
    /// Enable MLP layer hooks.
    pub hook_mlp: bool,
    /// Enable RMSNorm layer hooks.
    pub hook_rmsnorm: bool,
}

impl HookConfig {
    pub fn default_stuartian() -> Self {
        Self {
            max_hooks: 64,
            activation_threshold: 0.1,
            hook_attention: true,
            hook_mlp: true,
            hook_rmsnorm: false,
        }
    }
}

impl Default for HookConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

/// Captured activation from a transformer layer.
#[derive(Debug, Clone)]
pub struct ActivationCapture {
    /// Layer identifier (e.g., "layer_3.attention.output").
    pub layer: String,
    /// Activation tensor (flattened).
    pub activations: Vec<f64>,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Model identifier.
    pub model_id: String,
}

/// Activation Hook — intercepts transformer layer outputs.
#[derive(Debug, Clone)]
pub struct ActivationHook {
    config: HookConfig,
    captures: Vec<ActivationCapture>,
    active_count: usize,
}

impl ActivationHook {
    pub fn new() -> Self {
        Self {
            config: HookConfig::default(),
            captures: Vec::new(),
            active_count: 0,
        }
    }

    pub fn with_config(config: HookConfig) -> Self {
        Self {
            config,
            captures: Vec::new(),
            active_count: 0,
        }
    }

    /// Register a hook on a specific layer.
    pub fn register_layer(&mut self, layer: &str) -> Result<(), HookError> {
        if self.active_count >= self.config.max_hooks {
            return Err(HookError::HookLimitExceeded);
        }

        // Validate layer type
        let is_valid = if layer.contains("attention") && self.config.hook_attention {
            true
        } else if layer.contains("mlp") && self.config.hook_mlp {
            true
        } else if layer.contains("rmsnorm") && self.config.hook_rmsnorm {
            true
        } else {
            false
        };

        if !is_valid {
            return Err(HookError::LayerNotFound {
                layer: layer.to_string(),
            });
        }

        self.active_count += 1;
        Ok(())
    }

    /// Capture activations from a layer.
    pub fn capture(
        &mut self,
        layer: &str,
        activations: Vec<f64>,
        timestamp_ms: u64,
        model_id: &str,
    ) -> Result<ActivationCapture, HookError> {
        let capture = ActivationCapture {
            layer: layer.to_string(),
            activations,
            timestamp_ms,
            model_id: model_id.to_string(),
        };

        self.captures.push(capture.clone());
        Ok(capture)
    }

    /// Get captures above activation threshold.
    pub fn get_significant(&self, threshold: Option<f64>) -> Vec<&ActivationCapture> {
        let threshold = threshold.unwrap_or(self.config.activation_threshold);
        self.captures
            .iter()
            .filter(|c| c.activations.iter().any(|a| a.abs() > threshold))
            .collect()
    }

    /// Get capture count.
    pub fn capture_count(&self) -> usize {
        self.captures.len()
    }

    /// Get active hook count.
    pub fn active_hook_count(&self) -> usize {
        self.active_count
    }

    /// Clear all captures.
    pub fn clear(&mut self) {
        self.captures.clear();
    }
}

impl Default for ActivationHook {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ActivationHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ActivationHook {{ active: {}, captures: {} }}",
            self.active_count,
            self.captures.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_creation() {
        let hook = ActivationHook::new();
        assert_eq!(hook.active_hook_count(), 0);
        assert_eq!(hook.capture_count(), 0);
    }

    #[test]
    fn test_register_attention_layer() {
        let mut hook = ActivationHook::new();
        hook.register_layer("layer_3.attention.output").unwrap();
        assert_eq!(hook.active_hook_count(), 1);
    }

    #[test]
    fn test_register_mlp_layer() {
        let mut hook = ActivationHook::new();
        hook.register_layer("layer_5.mlp.gate").unwrap();
        assert_eq!(hook.active_hook_count(), 1);
    }

    #[test]
    fn test_register_disabled_layer() {
        let mut hook = ActivationHook::new();
        // RMSNorm is disabled by default
        let result = hook.register_layer("layer_1.rmsnorm");
        assert!(result.is_err());
    }

    #[test]
    fn test_hook_limit() {
        let mut hook = ActivationHook::with_config(HookConfig {
            max_hooks: 2,
            ..HookConfig::default()
        });
        hook.register_layer("layer_1.attention.output").unwrap();
        hook.register_layer("layer_2.attention.output").unwrap();
        let result = hook.register_layer("layer_3.attention.output");
        assert_eq!(result, Err(HookError::HookLimitExceeded));
    }

    #[test]
    fn test_capture_activations() {
        let mut hook = ActivationHook::new();
        let capture = hook
            .capture("layer_3.attention", vec![0.5, 0.8, 0.2], 1000, "qwen-7b")
            .unwrap();
        assert_eq!(capture.layer, "layer_3.attention");
        assert_eq!(hook.capture_count(), 1);
    }

    #[test]
    fn test_get_significant() {
        let mut hook = ActivationHook::new();
        hook.capture("layer_1", vec![0.05, 0.02], 1000, "m")
            .unwrap(); // below threshold
        hook.capture("layer_2", vec![0.5, 0.8], 2000, "m").unwrap(); // above threshold
        let significant = hook.get_significant(None);
        assert_eq!(significant.len(), 1);
    }

    #[test]
    fn test_clear_captures() {
        let mut hook = ActivationHook::new();
        hook.capture("layer_1", vec![0.5], 1000, "m").unwrap();
        hook.clear();
        assert_eq!(hook.capture_count(), 0);
    }

    #[test]
    fn test_hook_display() {
        let hook = ActivationHook::new();
        let s = format!("{}", hook);
        assert!(s.contains("ActivationHook"));
    }

    #[test]
    fn test_default_config() {
        let config = HookConfig::default();
        assert!(config.hook_attention);
        assert!(config.hook_mlp);
        assert!(!config.hook_rmsnorm);
    }
}
