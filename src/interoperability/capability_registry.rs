//! Capability Registry - Registro de capacidades por modelo con negociación de esquema
//!
//! Implementa un registro basado en capacidades para enrutamiento adaptativo
//! entre modelos en la red ed2kIA. Permite:
//! 1. Registro de modelos con capacidades, latencia y memoria
//! 2. Búsqueda por tarea, versión de esquema o criterios óptimos
//! 3. Negociación de esquema entre múltiples modelos
//! 4. Estadísticas del registro
//!
//! **Feature:** `v1.1-sprint1`

#[cfg(feature = "v1.1-sprint1")]
mod impl_ {
    use serde::{Deserialize, Serialize};
    use std::collections::{HashMap, HashSet};
    use thiserror::Error;
    use tracing::{debug, info, warn};

    // ============================================================================
    // Errors
    // ============================================================================

    /// Error específico del Capability Registry
    #[derive(Debug, Error, PartialEq)]
    pub enum RegistryError {
        #[error("Duplicate model registration: {model_id}")]
        DuplicateModel { model_id: String },

        #[error("Model not found: {model_id}")]
        ModelNotFound { model_id: String },

        #[error("No compatible schema found among registered models")]
        NoCompatibleSchema,
    }

    // ============================================================================
    // Model Capability
    // ============================================================================

    /// Descripción de las capacidades de un modelo registrado
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ModelCapability {
        /// Identificador único del modelo en el registro
        pub model_id: String,
        /// Nombre del modelo (e.g., "qwen-scope-7b", "llama-3-8b")
        pub model_name: String,
        /// Versión del esquema de serialización (e.g., "1.0.0")
        pub schema_version: String,
        /// Tareas soportadas (e.g., ["sae_forward", "feature_extraction", "embedding"])
        pub supported_tasks: Vec<String>,
        /// Dimensionalidad de entrada esperada
        pub input_dim: usize,
        /// Dimensionalidad de salida producida
        pub output_dim: usize,
        /// Tamaño máximo de batch soportado
        pub max_batch_size: usize,
        /// Latencia percentil 50 en milisegundos
        pub latency_p50_ms: f64,
        /// Latencia percentil 99 en milisegundos
        pub latency_p99_ms: f64,
        /// Huella de memoria en megabytes
        pub memory_footprint_mb: usize,
    }

    impl ModelCapability {
        /// Crear una nueva capacidad de modelo
        #[allow(clippy::too_many_arguments)]
        pub fn new(
            model_id: String,
            model_name: String,
            schema_version: String,
            supported_tasks: Vec<String>,
            input_dim: usize,
            output_dim: usize,
            max_batch_size: usize,
            latency_p50_ms: f64,
            latency_p99_ms: f64,
            memory_footprint_mb: usize,
        ) -> Self {
            Self {
                model_id,
                model_name,
                schema_version,
                supported_tasks,
                input_dim,
                output_dim,
                max_batch_size,
                latency_p50_ms,
                latency_p99_ms,
                memory_footprint_mb,
            }
        }

        /// Verificar si el modelo soporta una tarea dada
        pub fn supports_task(&self, task: &str) -> bool {
            self.supported_tasks.iter().any(|t| t == task)
        }
    }

    // ============================================================================
    // Registry Stats
    // ============================================================================

    /// Estadísticas agregadas del registro de capacidades
    #[derive(Debug, Clone)]
    pub struct RegistryStats {
        /// Total de modelos registrados
        pub total_models: usize,
        /// Número de tareas únicas soportadas
        pub unique_tasks: usize,
        /// Número de versiones de esquema únicas
        pub schema_versions: usize,
        /// Latencia promedio p50 en milisegundos
        pub avg_latency_ms: f64,
    }

    // ============================================================================
    // Capability Registry
    // ============================================================================

    /// Registro centralizado de capacidades por modelo
    ///
    /// Permite registrar, buscar y negociar capacidades entre modelos
    /// para enrutamiento adaptativo en la red ed2kIA.
    pub struct CapabilityRegistry {
        models: HashMap<String, ModelCapability>,
    }

    impl CapabilityRegistry {
        /// Crear un nuevo registro vacío
        pub fn new() -> Self {
            info!("Creating new CapabilityRegistry");
            Self {
                models: HashMap::new(),
            }
        }

        /// Registrar un modelo con sus capacidades
        ///
        /// # Errors
        /// Retorna `RegistryError::DuplicateModel` si el `model_id` ya existe
        pub fn register(&mut self, capability: ModelCapability) -> Result<(), RegistryError> {
            let model_id = capability.model_id.clone();
            if self.models.contains_key(&model_id) {
                warn!("Duplicate registration attempt for model: {}", model_id);
                return Err(RegistryError::DuplicateModel { model_id });
            }
            info!(
                "Registering model {} ({}) with {} tasks",
                capability.model_id,
                capability.model_name,
                capability.supported_tasks.len()
            );
            self.models.insert(model_id, capability);
            Ok(())
        }

        /// Desregistrar un modelo por su ID
        ///
        /// Retorna `true` si el modelo fue encontrado y eliminado
        pub fn unregister(&mut self, model_id: &str) -> bool {
            if self.models.remove(model_id).is_some() {
                info!("Unregistered model: {}", model_id);
                true
            } else {
                debug!("Model not found for unregistration: {}", model_id);
                false
            }
        }

        /// Buscar modelos que soporten una tarea específica
        pub fn find_by_task(&self, task: &str) -> Vec<&ModelCapability> {
            let mut results: Vec<&ModelCapability> = self
                .models
                .values()
                .filter(|m| m.supports_task(task))
                .collect();
            results.sort_by(|a, b| a.latency_p50_ms.partial_cmp(&b.latency_p50_ms).unwrap());
            debug!(
                "Found {} models supporting task '{}'",
                results.len(),
                task
            );
            results
        }

        /// Buscar modelos con una versión de esquema específica
        pub fn find_by_schema(&self, schema_version: &str) -> Vec<&ModelCapability> {
            let mut results: Vec<&ModelCapability> = self
                .models
                .values()
                .filter(|m| m.schema_version == schema_version)
                .collect();
            results.sort_by(|a, b| a.model_id.cmp(&b.model_id));
            debug!(
                "Found {} models with schema version '{}'",
                results.len(),
                schema_version
            );
            results
        }

        /// Encontrar el modelo óptimo para una tarea dada con restricciones de latencia y memoria
        ///
        /// Selecciona el modelo con menor latencia p50 que cumpla los presupuestos
        pub fn find_optimal(
            &self,
            task: &str,
            latency_target_ms: f64,
            memory_limit_mb: usize,
        ) -> Option<&ModelCapability> {
            let candidates: Vec<&ModelCapability> = self
                .models
                .values()
                .filter(|m| {
                    m.supports_task(task)
                        && m.latency_p50_ms <= latency_target_ms
                        && m.memory_footprint_mb <= memory_limit_mb
                })
                .collect();

            if candidates.is_empty() {
                debug!(
                    "No optimal model found for task '{}' within latency={}ms, memory={}mb",
                    task, latency_target_ms, memory_limit_mb
                );
                return None;
            }

            let best = candidates
                .iter()
                .min_by(|a, b| a.latency_p50_ms.partial_cmp(&b.latency_p50_ms).unwrap())
                .copied();

            if let Some(m) = best {
                info!(
                    "Selected optimal model '{}' for task '{}' (latency={}ms, memory={}mb)",
                    m.model_name, task, m.latency_p50_ms, m.memory_footprint_mb
                );
            }

            best
        }

        /// Negociar una versión de esquema común entre un conjunto de modelos
        ///
        /// Retorna la versión de esquema más utilizada entre los modelos proporcionados,
        /// o `None` si la lista está vacía
        pub fn negotiate_schema(&self, models: &[&ModelCapability]) -> Option<String> {
            if models.is_empty() {
                warn!("Cannot negotiate schema: empty model list");
                return None;
            }

            let mut version_count: HashMap<String, usize> = HashMap::new();
            for m in models {
                *version_count
                    .entry(m.schema_version.clone())
                    .or_insert(0) += 1;
            }

            let best = version_count
                .into_iter()
                .max_by(|a, b| a.1.cmp(&b.1))
                .map(|(version, count)| {
                    info!(
                        "Negotiated schema version '{}' (used by {} models)",
                        version, count
                    );
                    version
                });

            best
        }

        /// Obtener referencias a todos los modelos registrados
        pub fn get_all(&self) -> Vec<&ModelCapability> {
            let mut all: Vec<&ModelCapability> = self.models.values().collect();
            all.sort_by(|a, b| a.model_id.cmp(&b.model_id));
            all
        }

        /// Calcular estadísticas agregadas del registro
        pub fn stats(&self) -> RegistryStats {
            let total = self.models.len();
            if total == 0 {
                return RegistryStats {
                    total_models: 0,
                    unique_tasks: 0,
                    schema_versions: 0,
                    avg_latency_ms: 0.0,
                };
            }

            let mut tasks = HashSet::new();
            let mut schemas = HashSet::new();
            let mut total_latency = 0.0;

            for m in self.models.values() {
                for t in &m.supported_tasks {
                    tasks.insert(t.clone());
                }
                schemas.insert(m.schema_version.clone());
                total_latency += m.latency_p50_ms;
            }

            RegistryStats {
                total_models: total,
                unique_tasks: tasks.len(),
                schema_versions: schemas.len(),
                avg_latency_ms: total_latency / total as f64,
            }
        }
    }

    impl Default for CapabilityRegistry {
        fn default() -> Self {
            Self::new()
        }
    }

    // ============================================================================
    // Unit Tests
    // ============================================================================

    #[cfg(test)]
    mod tests {
        use super::*;

        fn sample_capability(id: &str, name: &str, tasks: &[&str], latency: f64, memory: usize) -> ModelCapability {
            ModelCapability::new(
                id.to_string(),
                name.to_string(),
                "1.0.0".to_string(),
                tasks.iter().map(|s| s.to_string()).collect(),
                4096,
                22528,
                32,
                latency,
                latency * 2.5,
                memory,
            )
        }

        #[test]
        fn test_register_and_lookup() {
            let mut registry = CapabilityRegistry::new();

            let cap = sample_capability("m1", "qwen-scope-7b", &["sae_forward", "embedding"], 12.0, 512);
            assert!(registry.register(cap).is_ok());

            let cap2 = sample_capability("m2", "llama-3-8b", &["feature_extraction"], 15.0, 768);
            assert!(registry.register(cap2).is_ok());

            assert_eq!(registry.get_all().len(), 2);
        }

        #[test]
        fn test_duplicate_registration_fails() {
            let mut registry = CapabilityRegistry::new();
            let cap = sample_capability("m1", "model", &["task_a"], 10.0, 256);
            assert!(registry.register(cap.clone()).is_ok());
            assert_eq!(
                registry.register(cap),
                Err(RegistryError::DuplicateModel { model_id: "m1".to_string() })
            );
        }

        #[test]
        fn test_unregister() {
            let mut registry = CapabilityRegistry::new();
            let cap = sample_capability("m1", "model", &["task_a"], 10.0, 256);
            registry.register(cap).unwrap();

            assert!(registry.unregister("m1"));
            assert!(!registry.unregister("nonexistent"));
            assert_eq!(registry.get_all().len(), 0);
        }

        #[test]
        fn test_find_by_task() {
            let mut registry = CapabilityRegistry::new();
            registry.register(sample_capability("m1", "model_a", &["sae_forward", "embedding"], 10.0, 256)).unwrap();
            registry.register(sample_capability("m2", "model_b", &["feature_extraction"], 15.0, 512)).unwrap();
            registry.register(sample_capability("m3", "model_c", &["sae_forward", "embedding"], 8.0, 128)).unwrap();

            let sae_models = registry.find_by_task("sae_forward");
            assert_eq!(sae_models.len(), 2);
            // Sorted by latency: m3 (8.0) before m1 (10.0)
            assert_eq!(sae_models[0].model_id, "m3");
            assert_eq!(sae_models[1].model_id, "m1");

            let feat_models = registry.find_by_task("feature_extraction");
            assert_eq!(feat_models.len(), 1);
            assert_eq!(feat_models[0].model_id, "m2");
        }

        #[test]
        fn test_find_by_schema() {
            let mut registry = CapabilityRegistry::new();
            registry.register(sample_capability("m1", "model_a", &["task_a"], 10.0, 256)).unwrap();

            let mut cap_v2 = sample_capability("m2", "model_b", &["task_a"], 12.0, 384);
            cap_v2.schema_version = "2.0.0".to_string();
            registry.register(cap_v2).unwrap();

            let v1_models = registry.find_by_schema("1.0.0");
            assert_eq!(v1_models.len(), 1);
            assert_eq!(v1_models[0].model_id, "m1");

            let v2_models = registry.find_by_schema("2.0.0");
            assert_eq!(v2_models.len(), 1);
            assert_eq!(v2_models[0].model_id, "m2");
        }

        #[test]
        fn test_find_optimal() {
            let mut registry = CapabilityRegistry::new();
            registry.register(sample_capability("m1", "fast_model", &["sae_forward"], 8.0, 128)).unwrap();
            registry.register(sample_capability("m2", "slow_model", &["sae_forward"], 20.0, 1024)).unwrap();
            registry.register(sample_capability("m3", "heavy_model", &["sae_forward"], 5.0, 2048)).unwrap();

            // Within budget: latency <= 10ms, memory <= 256mb -> m1
            let optimal = registry.find_optimal("sae_forward", 10.0, 256);
            assert!(optimal.is_some());
            assert_eq!(optimal.unwrap().model_id, "m1");

            // Tighter budget: no match
            let none = registry.find_optimal("sae_forward", 3.0, 64);
            assert!(none.is_none());
        }

        #[test]
        fn test_negotiate_schema() {
            let mut registry = CapabilityRegistry::new();
            registry.register(sample_capability("m1", "model_a", &["task_a"], 10.0, 256)).unwrap();
            registry.register(sample_capability("m2", "model_b", &["task_a"], 12.0, 384)).unwrap();

            let mut cap_v2 = sample_capability("m3", "model_c", &["task_a"], 15.0, 512);
            cap_v2.schema_version = "2.0.0".to_string();
            registry.register(cap_v2).unwrap();

            let all = registry.get_all();
            let models: Vec<&ModelCapability> = all.into_iter().collect();

            // Most common is "1.0.0" (2 models)
            let negotiated = registry.negotiate_schema(&models);
            assert_eq!(negotiated, Some("1.0.0".to_string()));
        }

        #[test]
        fn test_negotiate_schema_empty() {
            let registry = CapabilityRegistry::new();
            assert!(registry.negotiate_schema(&[]).is_none());
        }

        #[test]
        fn test_stats() {
            let mut registry = CapabilityRegistry::new();
            registry.register(sample_capability("m1", "model_a", &["sae_forward", "embedding"], 10.0, 256)).unwrap();
            registry.register(sample_capability("m2", "model_b", &["feature_extraction", "embedding"], 20.0, 512)).unwrap();

            let stats = registry.stats();
            assert_eq!(stats.total_models, 2);
            assert_eq!(stats.unique_tasks, 3); // sae_forward, embedding, feature_extraction
            assert_eq!(stats.schema_versions, 1);
            assert!((stats.avg_latency_ms - 15.0).abs() < f64::EPSILON);
        }

        #[test]
        fn test_stats_empty() {
            let registry = CapabilityRegistry::new();
            let stats = registry.stats();
            assert_eq!(stats.total_models, 0);
            assert_eq!(stats.unique_tasks, 0);
            assert_eq!(stats.schema_versions, 0);
            assert_eq!(stats.avg_latency_ms, 0.0);
        }
    }
}

#[cfg(feature = "v1.1-sprint1")]
pub use impl_::*;
