//! Distributed SAE Training Pipeline — Loop de entrenamiento distribuido compatible con candle-core.
//!
//! Fases por paso: forward pass → sparsity mask → backward pass → gradient clipping →
//! compresión INT8 → envío al agregador.
//! Checkpointing automático cada N pasos. Hooks de validación: on_step, on_epoch, on_convergence.
//!
//! Feature gate: `#[cfg(feature = "v2.1-sae-training")]`

use candle_core::{Device, DType, Module, Tensor, D};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ─── Errors ───

#[derive(Debug, Clone, PartialEq)]
pub enum TrainingError {
    ForwardPassFailed(String),
    BackwardPassFailed(String),
    GradientClippingFailed(String),
    CheckpointSaveFailed(String),
    CheckpointLoadFailed(String),
    InvalidConfig(String),
    DeviceError(String),
}

impl std::fmt::Display for TrainingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrainingError::ForwardPassFailed(msg) => write!(f, "Forward pass fallido: {}", msg),
            TrainingError::BackwardPassFailed(msg) => write!(f, "Backward pass fallido: {}", msg),
            TrainingError::GradientClippingFailed(msg) => {
                write!(f, "Gradient clipping fallido: {}", msg)
            }
            TrainingError::CheckpointSaveFailed(msg) => {
                write!(f, "Checkpoint guardado fallido: {}", msg)
            }
            TrainingError::CheckpointLoadFailed(msg) => {
                write!(f, "Checkpoint cargado fallido: {}", msg)
            }
            TrainingError::InvalidConfig(msg) => write!(f, "Configuración inválida: {}", msg),
            TrainingError::DeviceError(msg) => write!(f, "Error de dispositivo: {}", msg),
        }
    }
}

// ─── TrainingConfig ───

/// Configuración del pipeline de entrenamiento.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    /// Learning rate base.
    pub learning_rate: f64,
    /// Tamaño del batch.
    pub batch_size: usize,
    /// Número máximo de épocas.
    pub max_epochs: usize,
    /// Umbral de sparsity (0.0 - 1.0).
    pub sparsity_threshold: f32,
    /// Límite de gradient clipping (L2 norm).
    pub gradient_clip_norm: f32,
    /// Intervalo de checkpointing en pasos.
    pub checkpoint_interval: usize,
    /// Directorio de checkpoints.
    pub checkpoint_dir: String,
    /// Tolerancia de convergencia (cambio mínimo en loss).
    pub convergence_tolerance: f64,
    /// Patiencia para early stopping.
    pub early_stopping_patience: usize,
    /// Compresión INT8 para gradientes.
    pub int8_compression: bool,
}

impl TrainingConfig {
    pub fn new(batch_size: usize, max_epochs: usize) -> Self {
        Self {
            learning_rate: 1e-4,
            batch_size,
            max_epochs,
            sparsity_threshold: 0.9,
            gradient_clip_norm: 1.0,
            checkpoint_interval: 100,
            checkpoint_dir: "./checkpoints".to_string(),
            convergence_tolerance: 1e-6,
            early_stopping_patience: 10,
            int8_compression: true,
        }
    }

    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.batch_size == 0 {
            return Err(TrainingError::InvalidConfig(
                "batch_size debe ser > 0".to_string(),
            ));
        }
        if self.max_epochs == 0 {
            return Err(TrainingError::InvalidConfig(
                "max_epochs debe ser > 0".to_string(),
            ));
        }
        if self.sparsity_threshold < 0.0 || self.sparsity_threshold > 1.0 {
            return Err(TrainingError::InvalidConfig(
                "sparsity_threshold debe estar en [0.0, 1.0]".to_string(),
            ));
        }
        if self.gradient_clip_norm <= 0.0 {
            return Err(TrainingError::InvalidConfig(
                "gradient_clip_norm debe ser > 0".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self::new(32, 100)
    }
}

// ─── Checkpoint ───

/// Checkpoint de estado de entrenamiento.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Epoch actual.
    pub epoch: usize,
    /// Paso actual.
    pub step: usize,
    /// Loss actual.
    pub loss: f64,
    /// Best loss registrado.
    pub best_loss: f64,
    /// Pesos serializados como floats (simplificado).
    pub weights: HashMap<String, Vec<f32>>,
    /// Timestamp.
    pub timestamp: u64,
}

impl Checkpoint {
    pub fn new(epoch: usize, step: usize, loss: f64, best_loss: f64) -> Self {
        Self {
            epoch,
            step,
            loss,
            best_loss,
            weights: HashMap::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

// ─── TrainingMetrics ───

/// Métricas de entrenamiento.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingMetrics {
    /// Epoch actual.
    pub epoch: usize,
    /// Paso actual.
    pub step: usize,
    /// Loss actual.
    pub loss: f64,
    /// Learning rate actual.
    pub learning_rate: f64,
    /// Sparsity ratio actual.
    pub sparsity_ratio: f32,
    /// Gradient norm antes de clipping.
    pub gradient_norm: f32,
    /// Gradient norm después de clipping.
    pub clipped_gradient_norm: f32,
    /// Tiempo del paso en ms.
    pub step_duration_ms: f64,
    /// Timestamp.
    pub timestamp: u64,
}

impl TrainingMetrics {
    pub fn new(
        epoch: usize,
        step: usize,
        loss: f64,
        learning_rate: f64,
        sparsity_ratio: f32,
        gradient_norm: f32,
        clipped_gradient_norm: f32,
        step_duration_ms: f64,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            epoch,
            step,
            loss,
            learning_rate,
            sparsity_ratio,
            gradient_norm,
            clipped_gradient_norm,
            step_duration_ms,
            timestamp: now,
        }
    }
}

// ─── Hooks ───

/// Hook de validación para eventos de entrenamiento.
pub trait TrainingHook: Send + Sync {
    /// Llamado después de cada paso.
    fn on_step(&self, _metrics: &TrainingMetrics) {}
    /// Llamado al final de cada época.
    fn on_epoch(&self, _epoch: usize, _avg_loss: f64) {}
    /// Llamado cuando se detecta convergencia.
    fn on_convergence(&self, _final_loss: f64, _epoch: usize, _step: usize) {}
}

// ─── SAE Model (Minimal) ───

/// Modelo SAE mínimo para entrenamiento distribuido.
///
/// En producción, este sería reemplazado por qwen_scope_sae u otro SAE real.
pub struct SaeModel {
    /// Matriz de encoding (w_enc).
    pub w_enc: Tensor,
    /// Matriz de decoding (w_dec).
    pub w_dec: Tensor,
    /// Bias de encoding.
    pub b_enc: Tensor,
    /// Bias de decoding.
    pub b_dec: Tensor,
    /// Dimensión SAE.
    pub d_sae: usize,
    /// Dimensión del modelo.
    pub d_model: usize,
}

impl SaeModel {
    /// Crear modelo SAE con pesos aleatorios (para scaffold).
    pub fn new(d_model: usize, d_sae: usize, device: &Device) -> Result<Self, TrainingError> {
        let scale = 1.0 / (d_model as f64).sqrt();

        let w_enc = Tensor::rand(
            -scale,
            scale,
            (d_sae, d_model),
            device,
        ).map_err(|e| TrainingError::DeviceError(e.to_string()))?;

        let w_dec = Tensor::rand(
            -scale,
            scale,
            (d_model, d_sae),
            device,
        ).map_err(|e| TrainingError::DeviceError(e.to_string()))?;

        let b_enc = Tensor::zeros(d_sae, DType::F32, device)
            .map_err(|e| TrainingError::DeviceError(e.to_string()))?;

        let b_dec = Tensor::zeros(d_model, DType::F32, device)
            .map_err(|e| TrainingError::DeviceError(e.to_string()))?;

        Ok(Self {
            w_enc,
            w_dec,
            b_enc,
            b_dec,
            d_sae,
            d_model,
        })
    }

    /// Forward pass: x → encoded → sparse → decoded.
    pub fn forward(&self, x: &Tensor) -> Result<Tensor, TrainingError> {
        // encoded = ReLU(w_enc @ x + b_enc)
        let encoded = self
            .w_enc
            .matmul(x)
            .map_err(|e| TrainingError::ForwardPassFailed(e.to_string()))?
            .add(&self.b_enc)
            .map_err(|e| TrainingError::ForwardPassFailed(e.to_string()))?
            .relu()
            .map_err(|e| TrainingError::ForwardPassFailed(e.to_string()))?;

        // decoded = w_dec @ encoded + b_dec
        let decoded = self
            .w_dec
            .matmul(&encoded)
            .map_err(|e| TrainingError::ForwardPassFailed(e.to_string()))?
            .add(&self.b_dec)
            .map_err(|e| TrainingError::ForwardPassFailed(e.to_string()))?;

        Ok(decoded)
    }

    /// Aplicar sparsity mask (top-k activation).
    pub fn apply_sparsity(&self, activations: &Tensor, threshold: f32) -> Result<Tensor, TrainingError> {
        // Top-k: mantener solo los valores por encima del percentil
        let flat = activations
            .flatten_all()
            .map_err(|e| TrainingError::ForwardPassFailed(e.to_string()))?;

        let k = ((1.0 - threshold) * self.d_sae as f64) as usize;
        let k = k.max(1).min(self.d_sae);

        // Top-k values
        let (top_values, _) = flat
            .topk(D::LAST, k)
            .map_err(|e| TrainingError::ForwardPassFailed(e.to_string()))?;

        let threshold_value = top_values
            .select(D::LAST, k as i64 - 1)
            .map_err(|e| TrainingError::ForwardPassFailed(e.to_string()))?;

        let mask = activations
            .ge(&threshold_value)
            .map_err(|e| TrainingError::ForwardPassFailed(e.to_string()))?;

        let mask_f32 = mask
            .to_dtype(DType::F32)
            .map_err(|e| TrainingError::ForwardPassFailed(e.to_string()))?;

        activations
            .broadcast_mul(&mask_f32)
            .map_err(|e| TrainingError::ForwardPassFailed(e.to_string()))
    }
}

// ─── TrainingPipeline ───

/// Pipeline de entrenamiento distribuido SAE.
pub struct TrainingPipeline {
    config: TrainingConfig,
    model: SaeModel,
    device: Device,
    current_epoch: usize,
    current_step: usize,
    current_loss: f64,
    best_loss: f64,
    patience_counter: usize,
    hooks: Vec<Arc<dyn TrainingHook>>,
    checkpoints: Vec<Checkpoint>,
    metrics_history: Vec<TrainingMetrics>,
}

impl TrainingPipeline {
    /// Crear nuevo pipeline de entrenamiento.
    pub fn new(
        model: SaeModel,
        config: TrainingConfig,
    ) -> Result<Self, TrainingError> {
        config.validate()?;
        let device = model.w_enc.device().clone();

        Ok(Self {
            config,
            model,
            device,
            current_epoch: 0,
            current_step: 0,
            current_loss: f64::MAX,
            best_loss: f64::MAX,
            patience_counter: 0,
            hooks: Vec::new(),
            checkpoints: Vec::new(),
            metrics_history: Vec::new(),
        })
    }

    /// Agregar hook de validación.
    pub fn add_hook(&mut self, hook: Arc<dyn TrainingHook>) {
        self.hooks.push(hook);
    }

    /// Calcular loss MSE entre predicción y target.
    fn compute_loss(&self, predicted: &Tensor, target: &Tensor) -> Result<f64, TrainingError> {
        let diff = predicted
            .sub(target)
            .map_err(|e| TrainingError::ForwardPassFailed(e.to_string()))?;
        let sq = diff
            .square()
            .map_err(|e| TrainingError::ForwardPassFailed(e.to_string()))?;
        let mean = sq
            .mean_all_d()
            .map_err(|e| TrainingError::ForwardPassFailed(e.to_string()))?;
        mean.to_scalar()
            .to_f64()
            .map_err(|e| TrainingError::ForwardPassFailed(e.to_string()))
    }

    /// Gradient clipping por L2 norm.
    fn clip_gradients(&self, gradients: &mut [f32], max_norm: f32) -> (f32, f32) {
        let mut norm = 0.0f32;
        for g in gradients {
            norm += g * g;
        }
        norm = norm.sqrt();

        let clipped_norm = if norm > max_norm {
            let scale = max_norm / norm;
            for g in gradients.iter_mut() {
                *g *= scale;
            }
            max_norm
        } else {
            norm
        };

        (norm, clipped_norm)
    }

    /// Compresión INT8 de gradientes.
    fn compress_int8(&self, gradients: &[f32]) -> Vec<i8> {
        if gradients.is_empty() {
            return Vec::new();
        }

        let min = gradients.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = gradients.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let range = (max - min).max(1e-8);

        gradients
            .iter()
            .map(|g| {
                let normalized = (*g - min) / range;
                (normalized * 255.0) as i8
            })
            .collect()
    }

    /// Descompresión INT8 a f32.
    fn decompress_int8(&self, compressed: &[i8], original_len: usize) -> Vec<f32> {
        if compressed.is_empty() || original_len == 0 {
            return vec![0.0; original_len];
        }

        compressed
            .iter()
            .take(original_len)
            .map(|&c| c as f32 / 255.0)
            .collect()
    }

    /// Ejecutar un paso de entrenamiento.
    pub fn train_step(&mut self, input: &Tensor, target: &Tensor) -> Result<TrainingMetrics, TrainingError> {
        let start = Instant::now();

        // Forward pass
        let predicted = self.model.forward(input)?;

        // Sparsity mask
        let _sparse = self.model.apply_sparsity(&predicted, self.config.sparsity_threshold)?;

        // Compute loss
        let loss = self.compute_loss(&predicted, target)?;
        self.current_loss = loss;

        // Simulated gradient extraction (en producción: backward pass real con candle-nn)
        let grad_dim = self.model.d_sae * self.model.d_model + self.model.d_model * self.model.d_sae;
        let mut gradients: Vec<f32> = (0..grad_dim)
            .map(|_| fastrand::f32() * 0.01) // simulated gradients
            .collect();

        // Gradient clipping
        let (grad_norm, clipped_norm) = self.clip_gradients(&mut gradients, self.config.gradient_clip_norm);

        // Compression
        let _compressed = if self.config.int8_compression {
            self.compress_int8(&gradients)
        } else {
            Vec::new()
        };

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

        let metrics = TrainingMetrics::new(
            self.current_epoch,
            self.current_step,
            loss,
            self.config.learning_rate,
            self.config.sparsity_threshold,
            grad_norm,
            clipped_norm,
            duration_ms,
        );

        // Update state
        self.current_step += 1;

        // Track best loss
        if loss < self.best_loss {
            self.best_loss = loss;
            self.patience_counter = 0;
        } else {
            self.patience_counter += 1;
        }

        // Emit on_step hook
        for hook in &self.hooks {
            hook.on_step(&metrics);
        }

        // Store metrics
        self.metrics_history.push(metrics.clone());

        // Checkpointing
        if self.current_step % self.config.checkpoint_interval == 0 {
            self.save_checkpoint()?;
        }

        Ok(metrics)
    }

    /// Ejecutar una época completa.
    pub fn train_epoch(
        &mut self,
        data: &[(&Tensor, &Tensor)],
    ) -> Result<f64, TrainingError> {
        if data.is_empty() {
            return Ok(0.0);
        }

        let mut total_loss = 0.0f64;

        for (input, target) in data {
            let metrics = self.train_step(input, target)?;
            total_loss += metrics.loss;
        }

        let avg_loss = total_loss / data.len() as f64;

        // Emit on_epoch hook
        for hook in &self.hooks {
            hook.on_epoch(self.current_epoch, avg_loss);
        }

        self.current_epoch += 1;
        Ok(avg_loss)
    }

    /// Ejecutar entrenamiento completo.
    pub fn fit(
        &mut self,
        data: &[(&Tensor, &Tensor)],
    ) -> Result<Vec<TrainingMetrics>, TrainingError> {
        let mut converged = false;

        for epoch in 0..self.config.max_epochs {
            let avg_loss = self.train_epoch(data)?;

            // Check convergence
            if self.patience_counter >= self.config.early_stopping_patience {
                for hook in &self.hooks {
                    hook.on_convergence(avg_loss, epoch, self.current_step);
                }
                converged = true;
                break;
            }

            // Check loss stability
            if self.current_loss.abs_diff(self.best_loss, self.config.convergence_tolerance) {
                for hook in &self.hooks {
                    hook.on_convergence(avg_loss, epoch, self.current_step);
                }
                converged = true;
                break;
            }
        }

        if !converged {
            // Final checkpoint
            self.save_checkpoint()?;
        }

        Ok(self.metrics_history.clone())
    }

    /// Guardar checkpoint.
    pub fn save_checkpoint(&self) -> Result<Checkpoint, TrainingError> {
        let checkpoint = Checkpoint::new(
            self.current_epoch,
            self.current_step,
            self.current_loss,
            self.best_loss,
        );

        self.checkpoints.push(checkpoint.clone());
        Ok(checkpoint)
    }

    /// Obtener último checkpoint.
    pub fn last_checkpoint(&self) -> Option<&Checkpoint> {
        self.checkpoints.last()
    }

    /// Obtener historial de métricas.
    pub fn metrics_history(&self) -> &[TrainingMetrics] {
        &self.metrics_history
    }

    /// Obtener configuración.
    pub fn config(&self) -> &TrainingConfig {
        &self.config
    }

    /// Obtener mejor loss.
    pub fn best_loss(&self) -> f64 {
        self.best_loss
    }

    /// Verificar si ha convergido.
    pub fn has_converged(&self) -> bool {
        self.patience_counter >= self.config.early_stopping_patience
    }
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_new() {
        let config = TrainingConfig::new(64, 50);
        assert_eq!(config.batch_size, 64);
        assert_eq!(config.max_epochs, 50);
        assert_eq!(config.learning_rate, 1e-4);
        assert_eq!(config.sparsity_threshold, 0.9);
        assert_eq!(config.gradient_clip_norm, 1.0);
        assert_eq!(config.checkpoint_interval, 100);
    }

    #[test]
    fn test_config_validate() {
        let config = TrainingConfig::new(32, 10);
        assert!(config.validate().is_ok());

        let mut bad_config = TrainingConfig::new(0, 10);
        assert!(bad_config.validate().is_err());

        let mut bad_config = TrainingConfig::new(32, 0);
        assert!(bad_config.validate().is_err());
    }

    #[test]
    fn test_config_default() {
        let config = TrainingConfig::default();
        assert_eq!(config.batch_size, 32);
        assert_eq!(config.max_epochs, 100);
    }

    #[test]
    fn test_checkpoint_new() {
        let cp = Checkpoint::new(5, 500, 0.023, 0.020);
        assert_eq!(cp.epoch, 5);
        assert_eq!(cp.step, 500);
        assert_eq!(cp.loss, 0.023);
        assert_eq!(cp.best_loss, 0.020);
        assert!(cp.timestamp > 0);
    }

    #[test]
    fn test_training_metrics_new() {
        let m = TrainingMetrics::new(1, 100, 0.05, 1e-4, 0.9, 1.5, 1.0, 50.0);
        assert_eq!(m.epoch, 1);
        assert_eq!(m.step, 100);
        assert_eq!(m.loss, 0.05);
        assert_eq!(m.learning_rate, 1e-4);
        assert_eq!(m.sparsity_ratio, 0.9);
        assert_eq!(m.gradient_norm, 1.5);
        assert_eq!(m.clipped_gradient_norm, 1.0);
        assert_eq!(m.step_duration_ms, 50.0);
    }

    #[test]
    fn test_clip_gradients_no_clip() {
        let pipeline = TrainingPipeline::new(
            SaeModel::new(128, 256, &Device::Cpu).unwrap(),
            TrainingConfig::default(),
        ).unwrap();

        let mut grads = vec![0.1, 0.2, 0.3];
        let (norm, clipped) = pipeline.clip_gradients(&mut grads, 10.0);
        assert!(norm < clipped || (norm - clipped).abs() < 1e-6);
    }

    #[test]
    fn test_clip_gradients_with_clip() {
        let pipeline = TrainingPipeline::new(
            SaeModel::new(128, 256, &Device::Cpu).unwrap(),
            TrainingConfig::default(),
        ).unwrap();

        let mut grads = vec![10.0, 20.0, 30.0];
        let (norm, clipped) = pipeline.clip_gradients(&mut grads, 5.0);
        assert!(norm > 5.0);
        assert!((clipped - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_compress_decompress_int8() {
        let pipeline = TrainingPipeline::new(
            SaeModel::new(128, 256, &Device::Cpu).unwrap(),
            TrainingConfig::default(),
        ).unwrap();

        let grads = vec![0.0, 0.5, 1.0, -0.5, -1.0];
        let compressed = pipeline.compress_int8(&grads);
        assert_eq!(compressed.len(), grads.len());

        let decompressed = pipeline.decompress_int8(&compressed, grads.len());
        assert_eq!(decompressed.len(), grads.len());
    }

    #[test]
    fn test_compress_empty() {
        let pipeline = TrainingPipeline::new(
            SaeModel::new(128, 256, &Device::Cpu).unwrap(),
            TrainingConfig::default(),
        ).unwrap();

        let compressed = pipeline.compress_int8(&[]);
        assert!(compressed.is_empty());
    }

    #[test]
    fn test_sae_model_new() {
        let model = SaeModel::new(128, 256, &Device::Cpu).unwrap();
        assert_eq!(model.d_sae, 256);
        assert_eq!(model.d_model, 128);
    }

    #[test]
    fn test_pipeline_new() {
        let model = SaeModel::new(128, 256, &Device::Cpu).unwrap();
        let config = TrainingConfig::new(32, 10);
        let pipeline = TrainingPipeline::new(model, config).unwrap();
        assert_eq!(pipeline.best_loss(), f64::MAX);
        assert!(!pipeline.has_converged());
    }

    #[test]
    fn test_pipeline_invalid_config() {
        let model = SaeModel::new(128, 256, &Device::Cpu).unwrap();
        let mut config = TrainingConfig::new(0, 10);
        let result = TrainingPipeline::new(model, config);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_display() {
        let err = TrainingError::ForwardPassFailed("test".to_string());
        assert!(format!("{}", err).contains("Forward pass"));

        let err = TrainingError::BackwardPassFailed("test".to_string());
        assert!(format!("{}", err).contains("Backward pass"));

        let err = TrainingError::CheckpointSaveFailed("test".to_string());
        assert!(format!("{}", err).contains("Checkpoint guardado"));

        let err = TrainingError::InvalidConfig("test".to_string());
        assert!(format!("{}", err).contains("Configuración inválida"));
    }

    struct TestHook {
        pub steps: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl TrainingHook for TestHook {
        fn on_step(&self, _metrics: &TrainingMetrics) {
            self.steps.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    #[test]
    fn test_add_hook() {
        let model = SaeModel::new(16, 32, &Device::Cpu).unwrap();
        let mut pipeline = TrainingPipeline::new(model, TrainingConfig::default()).unwrap();

        let hook = Arc::new(TestHook {
            steps: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        });
        pipeline.add_hook(hook.clone());

        assert_eq!(pipeline.hooks.len(), 1);
    }
}
