//! Schema - Esquema de normalización de hidden states (Qwen-Scope format)
//!
//! Define el esquema canónico para intercambio de hidden states
//! entre modelos en la red ed2kIA.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use super::adapter::{NormalizedHiddenState, SourceModel};

/// Versión del esquema de normalización
pub const SCHEMA_VERSION: &str = "1.0.0";

/// Esquema canónico Qwen-Scope para hidden states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QwenScopeSchema {
    /// Versión del esquema
    pub version: String,
    /// Dimensionalidad canónica
    pub canonical_dim: usize,
    /// Tipo de normalización requerida
    pub required_norm: String,
    /// Rango de valores esperado después de normalización
    pub value_range: (f32, f32),
    /// Formato de serialización (flat_f32, sparse_indices)
    pub serialization_format: String,
    /// Mapeo de modelos soportados → reglas de adaptación
    pub model_rules: HashMap<SourceModel, AdaptationRule>,
}

/// Regla de adaptación para un modelo específico
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationRule {
    /// Dimensionalidad de entrada esperada
    pub input_dim: usize,
    /// Capas soportadas (None = todas)
    pub supported_layers: Option<Vec<u32>>,
    /// Normalización requerida antes de proyección
    pub pre_norm: String,
    /// Método de proyección (truncate, pad, linear)
    pub projection_method: String,
}

impl QwenScopeSchema {
    /// Crear esquema por defecto (Qwen2-7B compatible)
    pub fn default_qwen_scope() -> Self {
        let mut model_rules = HashMap::new();

        model_rules.insert(
            SourceModel::Llama,
            AdaptationRule {
                input_dim: 4096,
                supported_layers: None,
                pre_norm: "rmsnorm".to_string(),
                projection_method: "truncate".to_string(),
            },
        );

        model_rules.insert(
            SourceModel::Mistral,
            AdaptationRule {
                input_dim: 4096,
                supported_layers: None,
                pre_norm: "rmsnorm".to_string(),
                projection_method: "truncate".to_string(),
            },
        );

        model_rules.insert(
            SourceModel::Qwen,
            AdaptationRule {
                input_dim: 3584,
                supported_layers: None,
                pre_norm: "rmsnorm".to_string(),
                projection_method: "identity".to_string(),
            },
        );

        model_rules.insert(
            SourceModel::GPT2,
            AdaptationRule {
                input_dim: 768,
                supported_layers: None,
                pre_norm: "layernorm".to_string(),
                projection_method: "pad".to_string(),
            },
        );

        Self {
            version: SCHEMA_VERSION.to_string(),
            canonical_dim: 3584,
            required_norm: "rmsnorm".to_string(),
            value_range: (-3.0, 3.0),
            serialization_format: "flat_f32".to_string(),
            model_rules,
        }
    }

    /// Validar que un hidden state cumple el esquema
    pub fn validate(&self, state: &NormalizedHiddenState) -> Result<(), SchemaValidationError> {
        // Verificar dimensión
        if state.normalized_dim != self.canonical_dim {
            return Err(SchemaValidationError::DimensionMismatch {
                expected: self.canonical_dim,
                got: state.normalized_dim,
            });
        }

        // Verificar rango de valores
        for &val in &state.data {
            if val < self.value_range.0 || val > self.value_range.1 {
                return Err(SchemaValidationError::ValueOutOfRange {
                    value: val,
                    range: self.value_range,
                });
            }
        }

        // Verificar modelo soportado
        if !self.model_rules.contains_key(&state.source_model) {
            return Err(SchemaValidationError::UnsupportedModel(state.source_model.clone()));
        }

        Ok(())
    }

    /// Obtener regla de adaptación para un modelo
    pub fn get_rule(&self, model: &SourceModel) -> Option<&AdaptationRule> {
        self.model_rules.get(model)
    }

    /// Agregar regla personalizada
    pub fn add_rule(&mut self, model: SourceModel, rule: AdaptationRule) {
        info!("Agregando regla de adaptación para {}", model);
        self.model_rules.insert(model, rule);
    }
}

/// Errores de validación de esquema
#[derive(Debug, thiserror::Error)]
pub enum SchemaValidationError {
    #[error("Dimensión incorrecta: esperado {expected}, obtenido {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("Valor fuera de rango: {value} (rango: {range:?})")]
    ValueOutOfRange { value: f32, range: (f32, f32) },

    #[error("Modelo no soportado: {0:?}")]
    UnsupportedModel(SourceModel),

    #[error("Formato de serialización inválido: {0}")]
    InvalidFormat(String),
}

impl Default for QwenScopeSchema {
    fn default() -> Self {
        Self::default_qwen_scope()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interoperability::adapter::SourceModel;

    #[test]
    fn test_schema_creation() {
        let schema = QwenScopeSchema::default_qwen_scope();
        assert_eq!(schema.version, SCHEMA_VERSION);
        assert_eq!(schema.canonical_dim, 3584);
        assert_eq!(schema.model_rules.len(), 4);
    }

    #[test]
    fn test_schema_validate_valid_state() {
        let schema = QwenScopeSchema::default_qwen_scope();

        // Crear estado válido con dimensión canónica
        let state = NormalizedHiddenState::new(
            SourceModel::Qwen,
            0,
            vec![0.0f32; schema.canonical_dim],
        );

        assert!(schema.validate(&state).is_ok());
    }

    #[test]
    fn test_schema_validate_wrong_dimension() {
        let schema = QwenScopeSchema::default_qwen_scope();
        let state = NormalizedHiddenState::new(
            SourceModel::Qwen,
            0,
            vec![0.0f32; 100], // dim incorrecta
        );

        let err = schema.validate(&state).unwrap_err();
        assert!(matches!(err, SchemaValidationError::DimensionMismatch { .. }));
    }

    #[test]
    fn test_schema_validate_unsupported_model() {
        let schema = QwenScopeSchema::default_qwen_scope();
        let state = NormalizedHiddenState::new(
            SourceModel::Custom("unknown".to_string()),
            0,
            vec![0.0f32; schema.canonical_dim],
        );

        let err = schema.validate(&state).unwrap_err();
        assert!(matches!(err, SchemaValidationError::UnsupportedModel(_)));
    }

    #[test]
    fn test_schema_validate_value_out_of_range() {
        let schema = QwenScopeSchema::default_qwen_scope();
        let mut data = vec![0.0f32; schema.canonical_dim];
        data[0] = 10.0; // fuera de rango (-3, 3)

        let state = NormalizedHiddenState::new(
            SourceModel::Qwen,
            0,
            data,
        );

        let err = schema.validate(&state).unwrap_err();
        assert!(matches!(err, SchemaValidationError::ValueOutOfRange { .. }));
    }

    #[test]
    fn test_schema_add_custom_rule() {
        let mut schema = QwenScopeSchema::default();
        let custom_model = SourceModel::Custom("my-model".to_string());
        let rule = AdaptationRule {
            input_dim: 2048,
            supported_layers: Some(vec![0, 5, 10]),
            pre_norm: "rmsnorm".to_string(),
            projection_method: "linear".to_string(),
        };

        schema.add_rule(custom_model.clone(), rule);
        assert!(schema.get_rule(&custom_model).is_some());
    }
}
