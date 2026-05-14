//! SAE Loader - Carga y gestión de Sparse Autoencoders con Candle
//!
//! Carga modelos SAE desde archivos .safetensors y proporciona
//! la infraestructura para inferencia distribuida.

use anyhow::{Context, Result};
use candle_core::{DType, Device, Tensor};
// MIGRATION: safetensors 0.3 removed load module, use candle_core safetensors loading
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Configuración del SAE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SAEConfig {
    /// Dimensionalidad de entrada (hidden_dim del LLM)
    pub input_dim: usize,
    /// Dimensionalidad del espacio latente SAE
    pub latent_dim: usize,
    /// Número de capas en el SAE
    pub num_layers: usize,
    /// Tipo de activación (relu, gelu, topk, etc.)
    pub activation: String,
    /// Umbral de sparsity (0.0 - 1.0)
    pub sparsity_threshold: f64,
    /// Modelo LLM de origen (ej: "Qwen2-7B")
    pub source_model: String,
    /// Versión del SAE
    pub version: String,
}

impl Default for SAEConfig {
    fn default() -> Self {
        Self {
            input_dim: 4096,   // Qwen2-7B hidden size
            latent_dim: 16384, // 4x expansion típico
            num_layers: 1,
            activation: "topk".to_string(),
            sparsity_threshold: 0.9,
            source_model: "Qwen2-7B".to_string(),
            version: "1.0.0".to_string(),
        }
    }
}

/// Pesos del SAE cargados desde .safetensors
#[derive(Debug, Clone)]
pub struct SAEPweights {
    /// W_enc: [latent_dim, input_dim] - Matriz de codificación
    pub w_enc: Tensor,
    /// b_enc: [latent_dim] - Bias de codificación
    pub b_enc: Tensor,
    /// W_dec: [input_dim, latent_dim] - Matriz de decodificación
    pub w_dec: Tensor,
    /// b_dec: [input_dim] - Bias de decodificación
    pub b_dec: Tensor,
    /// b_proj: [input_dim] - Bias de proyección (dead neuron correction)
    pub b_proj: Option<Tensor>,
}

/// Modelo SAE completo
#[derive(Debug)]
pub struct SAEModel {
    pub config: SAEConfig,
    pub weights: SAEPweights,
    pub device: Device,
}

impl SAEModel {
    /// Ejecutar forward pass del SAE
    ///
    /// # Arguments
    /// * `input` - Tensor de entrada [batch_size, input_dim]
    ///
    /// # Returns
    /// Tensor de activaciones sparse [batch_size, latent_dim]
    pub fn forward(&self, input: &Tensor) -> Result<Tensor> {
        // Codificación: latent = ReLU(W_enc @ x + b_enc)
        let encoded = (self.weights.w_enc.matmul(input)? + &self.weights.b_enc)?;

        // Activación TopK para sparsity
        let sparse_activations = self.apply_topk(&encoded)?;

        // Decodificación: x_reconstructed = W_dec @ latent + b_dec
        // (Opcional, para cálculo de loss durante training)
        // let reconstructed = self.weights.w_dec.matmul(&sparse_activations)? + &self.weights.b_dec;

        Ok(sparse_activations)
    }

    /// Aplicar activación TopK para sparsity
    fn apply_topk(&self, activations: &Tensor) -> Result<Tensor> {
        // TODO: Phase 2 - Implementar TopK activation real con Candle
        // Por ahora, umbral simple como placeholder
        let threshold = self.config.sparsity_threshold;

        // Crear mask con valores por encima del umbral
        // TODO: Phase 2 - Implementar con candle_ops
        debug!("Aplicando sparsity threshold: {}", threshold);

        // Placeholder: retornar activaciones sin modificar
        // En Phase 2, implementar: mask = activations > threshold
        // result = activations * mask
        Ok(activations.clone())
    }

    /// Extraer sparse features con índices y valores
    pub fn extract_sparse_features(&self, input: &Tensor) -> Result<Vec<SparseFeature>> {
        let _activations = self.forward(input)?;

        // TODO: Phase 2 - Extraer índices no-cero y valores
        // Por ahora, placeholder
        Ok(vec![])
    }
}

/// Feature sparse extraída del SAE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseFeature {
    /// Índice del neuron activado
    pub neuron_index: u32,
    /// Valor de activación
    pub activation_value: f32,
    /// Importancia relativa
    pub importance: f32,
}

/// Loader de modelos SAE desde .safetensors
pub struct SAELoader {
    /// Path al archivo .safetensors
    model_path: PathBuf,
    /// Dispositivo de inferencia (CPU/GPU)
    device: Device,
    /// Modelo cargado
    model: Option<SAEModel>,
    /// Cache de tensores para reutilización
    tensor_cache: HashMap<String, Tensor>,
}

impl SAELoader {
    /// Crear nuevo loader
    pub fn new(model_path: impl Into<PathBuf>) -> Self {
        Self {
            model_path: model_path.into(),
            device: Self::detect_device(),
            model: None,
            tensor_cache: HashMap::new(),
        }
    }

    /// Detectar dispositivo disponible (CPU/GPU)
    fn detect_device() -> Device {
        // Intentar CUDA primero
        #[cfg(feature = "cuda")]
        {
            if let Ok(_) = Device::new_cuda(0) {
                info!("GPU CUDA detectada - usando aceleración");
                return Device::new_cuda(0).unwrap();
            }
        }

        // Intentar Metal (Apple Silicon)
        #[cfg(feature = "metal")]
        {
            if let Ok(_) = Device::new_metal(0) {
                info!("GPU Metal detectada - usando aceleración");
                return Device::new_metal(0).unwrap();
            }
        }

        info!("Usando CPU para inferencia");
        Device::Cpu
    }

    /// Cargar modelo SAE desde .safetensors
    pub async fn load(&mut self) -> Result<&SAEModel> {
        info!(
            "Cargando SAE desde: {} en dispositivo: {:?}",
            self.model_path.display(),
            self.device
        );

        // Verificar que el archivo existe
        if !self.model_path.exists() {
            warn!(
                "Archivo SAE no encontrado: {}. Usando modelo placeholder.",
                self.model_path.display()
            );
            return self.load_placeholder();
        }

        // Leer archivo .safetensors
        let data = std::fs::read(&self.model_path)
            .with_context(|| format!("No se pudo leer {}", self.model_path.display()))?;

        // MIGRATION: safetensors::load removed in 0.3, use candle_core safetensors loading directly
        // Cargar configuración
        let config = self.load_config_from_bytes(&data)?;

        // Cargar pesos
        let weights = self.load_weights_from_bytes(&data, &config)?;

        let model = SAEModel {
            config,
            weights,
            device: self.device.clone(),
        };

        info!(
            "SAE cargado: input_dim={}, latent_dim={}, layers={}",
            model.config.input_dim, model.config.latent_dim, model.config.num_layers
        );

        self.model = Some(model);
        Ok(self.model.as_ref().unwrap())
    }

    /// Cargar placeholder para testing
    fn load_placeholder(&mut self) -> Result<&SAEModel> {
        info!("Creando modelo SAE placeholder para testing");

        let config = SAEConfig::default();

        // Crear tensores placeholder con ceros
        let w_enc = Tensor::zeros(
            (config.latent_dim, config.input_dim),
            DType::F32,
            &self.device,
        )?;
        let b_enc = Tensor::zeros(config.latent_dim, DType::F32, &self.device)?;
        let w_dec = Tensor::zeros(
            (config.input_dim, config.latent_dim),
            DType::F32,
            &self.device,
        )?;
        let b_dec = Tensor::zeros(config.input_dim, DType::F32, &self.device)?;

        let weights = SAEPweights {
            w_enc,
            b_enc,
            w_dec,
            b_dec,
            b_proj: None,
        };

        let model = SAEModel {
            config,
            weights,
            device: self.device.clone(),
        };

        self.model = Some(model);
        Ok(self.model.as_ref().unwrap())
    }

    /// Cargar configuración del SAE
    // MIGRATION: safetensors::load::OwnedSafetensors removed in 0.3, use bytes directly
    fn load_config_from_bytes(
        &self,
        _data: &[u8],
    ) -> Result<SAEConfig> {
        // TODO: Phase 2 - Parsear config del header de safetensors
        // Por ahora, usar defaults
        Ok(SAEConfig::default())
    }

    /// Cargar pesos desde safetensors
    fn load_weights_from_bytes(
        &self,
        _data: &[u8],
        config: &SAEConfig,
    ) -> Result<SAEPweights> {
        // MIGRATION: Use candle_core Tensor::from_safetensors directly
        // For now, create placeholder tensors (file loading in Phase 2)
        let w_enc = Tensor::zeros(
            (config.latent_dim, config.input_dim),
            DType::F32,
            &self.device,
        )?;
        let b_enc = Tensor::zeros(config.latent_dim, DType::F32, &self.device)?;
        let w_dec = Tensor::zeros(
            (config.input_dim, config.latent_dim),
            DType::F32,
            &self.device,
        )?;
        let b_dec = Tensor::zeros(config.input_dim, DType::F32, &self.device)?;

        Ok(SAEPweights {
            w_enc,
            b_enc,
            w_dec,
            b_dec,
            b_proj: None,
        })
    }

    /// Obtener referencia al modelo cargado
    pub fn model(&self) -> Option<&SAEModel> {
        self.model.as_ref()
    }

    /// Obtener dispositivo
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Limpiar cache de tensores
    pub fn clear_cache(&mut self) {
        self.tensor_cache.clear();
        debug!("Tensor cache limpiado");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sae_config_default() {
        let config = SAEConfig::default();
        assert_eq!(config.input_dim, 4096);
        assert_eq!(config.latent_dim, 16384);
        assert_eq!(config.activation, "topk");
    }

    #[test]
    fn test_device_detection() {
        let device = SAELoader::detect_device();
        // CLEANUP: Device doesn't implement PartialEq; use pattern matching instead
        match device {
            Device::Cpu => {},
            Device::Cuda(_) => {},
            Device::Metal(_) => {},
        }
    }

    #[test]
    fn test_loader_creation() {
        let loader = SAELoader::new("/tmp/test.safetensors");
        assert!(loader.model().is_none());
    }
}
