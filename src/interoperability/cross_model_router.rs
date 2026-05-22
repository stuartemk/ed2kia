//! Cross-Model Router - Enrutamiento adaptativo con cadena de fallback y negociación de esquema
//!
//! Implementa un enrutador inteligente que selecciona el modelo óptimo para cada
//! solicitud basándose en:
//! 1. Capacidades del modelo (tareas soportadas)
//! 2. Presupuesto de latencia y memoria
//! 3. Negociación de versión de esquema
//! 4. Cadenas de fallback ordenadas por prioridad
//! 5. Balanceo de carga entre modelos compatibles
//!
//! **Feature:** `v1.1-sprint1`

#[cfg(feature = "v1.1-sprint1")]
mod impl_ {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use thiserror::Error;
    use tracing::{debug, info, warn};

    use super::super::capability_registry::{CapabilityRegistry, ModelCapability};

    // ============================================================================
    // Routing Priority
    // ============================================================================

    /// Prioridad de enrutamiento para ordenar solicitudes
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum RoutingPriority {
        /// Crítico: latencia mínima, sin fallback
        Critical,
        /// Alta: fallback limitado
        High,
        /// Normal: fallback estándar
        Normal,
        /// Baja: puede usar cualquier modelo disponible
        Low,
    }

    impl RoutingPriority {
        /// Máximo número de fallbacks permitidos por prioridad
        pub fn max_fallbacks(self) -> usize {
            match self {
                RoutingPriority::Critical => 0,
                RoutingPriority::High => 2,
                RoutingPriority::Normal => 5,
                RoutingPriority::Low => 10,
            }
        }
    }

    // ============================================================================
    // Routing Strategy
    // ============================================================================

    /// Estrategia de enrutamiento utilizada para una solicitud
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum RoutingStrategy {
        /// Enrutamiento directo al modelo seleccionado
        Direct,
        /// Enrutamiento con fallback activado
        Fallback,
        /// Balanceo de carga entre modelos compatibles
        LoadBalanced,
        /// Enrutamiento con traducción de esquema
        SchemaTranslated,
    }

    // ============================================================================
    // Schema Translation
    // ============================================================================

    /// Descripción de una traducción de esquema requerida
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SchemaTranslation {
        /// Versión de esquema de origen
        pub source_schema: String,
        /// Versión de esquema de destino
        pub target_schema: String,
        /// Descripción de la transformación aplicada
        pub translation_function: String,
    }

    impl SchemaTranslation {
        /// Crear una nueva traducción de esquema
        pub fn new(
            source_schema: String,
            target_schema: String,
            translation_function: String,
        ) -> Self {
            Self {
                source_schema,
                target_schema,
                translation_function,
            }
        }
    }

    // ============================================================================
    // Routing Request
    // ============================================================================

    /// Solicitud de enrutamiento hacia un modelo
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RoutingRequest {
        /// Tarea a ejecutar (e.g., "sae_forward", "embedding")
        pub task: String,
        /// Esquema de entrada esperado
        pub input_schema: String,
        /// Presupuesto máximo de latencia en milisegundos
        pub latency_budget_ms: f64,
        /// Presupuesto máximo de memoria en megabytes
        pub memory_budget_mb: usize,
        /// Prioridad de la solicitud
        pub priority: RoutingPriority,
    }

    impl RoutingRequest {
        /// Crear una nueva solicitud de enrutamiento
        pub fn new(
            task: String,
            input_schema: String,
            latency_budget_ms: f64,
            memory_budget_mb: usize,
            priority: RoutingPriority,
        ) -> Self {
            Self {
                task,
                input_schema,
                latency_budget_ms,
                memory_budget_mb,
                priority,
            }
        }
    }

    // ============================================================================
    // Routing Result
    // ============================================================================

    /// Resultado del enrutamiento con modelo seleccionado y cadena de fallback
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RoutingResult {
        /// Modelo seleccionado para la solicitud
        pub selected_model: String,
        /// Cadena ordenada de modelos de fallback
        pub fallback_chain: Vec<String>,
        /// Traducción de esquema si fue necesaria
        pub schema_translation: Option<SchemaTranslation>,
        /// Latencia estimada en milisegundos
        pub estimated_latency_ms: f64,
        /// Estrategia de enrutamiento utilizada
        pub routing_strategy: RoutingStrategy,
    }

    // ============================================================================
    // Routing Stats
    // ============================================================================

    /// Estadísticas agregadas del enrutador
    #[derive(Debug, Clone)]
    pub struct RoutingStats {
        /// Total de solicitudes procesadas
        pub total_requests: usize,
        /// Enrutamientos directos (sin fallback)
        pub direct_routes: usize,
        /// Enrutamientos con fallback
        pub fallback_routes: usize,
        /// Latencia promedio en milisegundos
        pub avg_latency_ms: f64,
        /// Traducciones de esquema realizadas
        pub schema_translations: usize,
    }

    // ============================================================================
    // Routing Error
    // ============================================================================

    /// Error de enrutamiento
    #[derive(Debug, Error, PartialEq)]
    pub enum RoutingError {
        #[error("No compatible model found for task '{task}' with schema '{schema}'")]
        NoCompatibleModel { task: String, schema: String },

        #[error("Schema mismatch: request '{request}' vs model '{model}'")]
        SchemaMismatch { request: String, model: String },

        #[error("All {count} fallbacks exhausted for task '{task}'")]
        AllFallbacksExhausted { task: String, count: usize },

        #[error("Budget exceeded: latency={latency}ms > budget={budget}ms or memory={memory}mb > budget={budget_mem}mb")]
        BudgetExceeded {
            latency: f64,
            budget: f64,
            memory: usize,
            budget_mem: usize,
        },
    }

    // ============================================================================
    // Runtime Model Status
    // ============================================================================

    /// Estado en tiempo de ejecución de un modelo
    #[derive(Debug, Clone)]
    struct RuntimeStatus {
        /// Última latencia medida en milisegundos
        latency_ms: f64,
        /// Última huella de memoria en megabytes
        memory_mb: usize,
        /// Contador de solicitudes para balanceo de carga
        request_count: usize,
    }

    // ============================================================================
    // Cross-Model Router
    // ============================================================================

    /// Enrutador adaptativo entre modelos
    ///
    /// Utiliza un `CapabilityRegistry` para seleccionar el modelo óptimo
    /// basándose en capacidades, métricas en tiempo de ejecución y presupuestos.
    pub struct CrossModelRouter {
        registry: CapabilityRegistry,
        runtime_status: HashMap<String, RuntimeStatus>,
        stats: RoutingStats,
    }

    impl CrossModelRouter {
        /// Crear un nuevo enrutador con el registro proporcionado
        pub fn new(registry: CapabilityRegistry) -> Self {
            info!(
                "Creating CrossModelRouter with {} registered models",
                registry.get_all().len()
            );
            Self {
                registry,
                runtime_status: HashMap::new(),
                stats: RoutingStats {
                    total_requests: 0,
                    direct_routes: 0,
                    fallback_routes: 0,
                    avg_latency_ms: 0.0,
                    schema_translations: 0,
                },
            }
        }

        /// Crear un enrutador con registro vacío
        pub fn with_defaults() -> Self {
            Self::new(CapabilityRegistry::new())
        }

        /// Enrutamiento principal basado en la solicitud
        ///
        /// Selecciona el modelo óptimo considerando:
        /// - Tarea soportada
        /// - Presupuesto de latencia y memoria
        /// - Compatibilidad de esquema
        /// - Métricas en tiempo de ejecución
        ///
        /// # Errors
        /// Retorna `RoutingError` si no se encuentra modelo compatible
        pub fn route(&mut self, request: RoutingRequest) -> Result<RoutingResult, RoutingError> {
            let task = request.task.clone();
            let schema = request.input_schema.clone();

            // Buscar candidatos por tarea
            let candidates = self.registry.find_by_task(&task);
            if candidates.is_empty() {
                warn!("No models found for task '{}'", task);
                return Err(RoutingError::NoCompatibleModel { task, schema });
            }

            // Filtrar por presupuesto
            let within_budget: Vec<&ModelCapability> = candidates
                .iter()
                .filter(|m| {
                    let runtime_latency = self
                        .runtime_status
                        .get(&m.model_id)
                        .map(|s| s.latency_ms)
                        .unwrap_or(m.latency_p50_ms);

                    let runtime_memory = self
                        .runtime_status
                        .get(&m.model_id)
                        .map(|s| s.memory_mb)
                        .unwrap_or(m.memory_footprint_mb);

                    runtime_latency <= request.latency_budget_ms
                        && runtime_memory <= request.memory_budget_mb
                })
                .copied()
                .collect();

            if within_budget.is_empty() {
                // Verificar si hay candidatos pero exceden presupuesto
                if let Some(first) = candidates.first() {
                    let latency = first.latency_p50_ms;
                    let memory = first.memory_footprint_mb;
                    warn!(
                        "All models exceed budget for task '{}': latency={}ms, memory={}mb",
                        task, latency, memory
                    );
                    return Err(RoutingError::BudgetExceeded {
                        latency,
                        budget: request.latency_budget_ms,
                        memory,
                        budget_mem: request.memory_budget_mb,
                    });
                }
                return Err(RoutingError::NoCompatibleModel { task, schema });
            }

            // Seleccionar modelo óptimo (menor latencia con balanceo de carga)
            let selected = within_budget
                .iter()
                .min_by(|a, b| {
                    let a_count = self
                        .runtime_status
                        .get(&a.model_id)
                        .map(|s| s.request_count)
                        .unwrap_or(0);
                    let b_count = self
                        .runtime_status
                        .get(&b.model_id)
                        .map(|s| s.request_count)
                        .unwrap_or(0);

                    // Primero por latencia, luego por carga (round-robin)
                    let latency_cmp = a.latency_p50_ms.partial_cmp(&b.latency_p50_ms).unwrap();
                    if latency_cmp != std::cmp::Ordering::Equal {
                        latency_cmp
                    } else {
                        a_count.cmp(&b_count)
                    }
                })
                .copied();

            let selected = match selected {
                Some(m) => m,
                None => {
                    return Err(RoutingError::NoCompatibleModel { task, schema });
                }
            };

            // Verificar compatibilidad de esquema
            let schema_translation = if selected.schema_version != request.input_schema {
                let negotiated = self.registry.negotiate_schema(&[selected]);
                if let Some(common_schema) = negotiated {
                    if common_schema == request.input_schema {
                        None
                    } else {
                        Some(SchemaTranslation::new(
                            request.input_schema.clone(),
                            selected.schema_version.clone(),
                            format!(
                                "Transform from {} to {} via adapter",
                                request.input_schema, selected.schema_version
                            ),
                        ))
                    }
                } else {
                    return Err(RoutingError::SchemaMismatch {
                        request: request.input_schema,
                        model: selected.schema_version.clone(),
                    });
                }
            } else {
                None
            };

            // Construir cadena de fallback (excluyendo el modelo seleccionado)
            let mut fallback_chain: Vec<String> = within_budget
                .iter()
                .filter(|m| m.model_id != selected.model_id)
                .map(|m| m.model_id.clone())
                .collect();
            fallback_chain.sort();

            // Determinar estrategia
            let routing_strategy = if schema_translation.is_some() {
                RoutingStrategy::SchemaTranslated
            } else if fallback_chain.len() > 1 {
                RoutingStrategy::LoadBalanced
            } else {
                RoutingStrategy::Direct
            };

            // Calcular latencia estimada
            let estimated_latency = self
                .runtime_status
                .get(&selected.model_id)
                .map(|s| s.latency_ms)
                .unwrap_or(selected.latency_p50_ms);

            // Actualizar estadísticas
            self.stats.total_requests += 1;
            if fallback_chain.is_empty() {
                self.stats.direct_routes += 1;
            } else {
                self.stats.fallback_routes += 1;
            }
            if schema_translation.is_some() {
                self.stats.schema_translations += 1;
            }
            self.stats.avg_latency_ms = (self.stats.avg_latency_ms
                * (self.stats.total_requests - 1) as f64
                + estimated_latency)
                / self.stats.total_requests as f64;

            // Actualizar contador de carga
            self.runtime_status
                .entry(selected.model_id.clone())
                .and_modify(|s| s.request_count += 1)
                .or_insert(RuntimeStatus {
                    latency_ms: selected.latency_p50_ms,
                    memory_mb: selected.memory_footprint_mb,
                    request_count: 1,
                });

            info!(
                "Routed task '{}' to model '{}' (strategy={:?}, latency={}ms)",
                task, selected.model_name, routing_strategy, estimated_latency
            );

            Ok(RoutingResult {
                selected_model: selected.model_id.clone(),
                fallback_chain,
                schema_translation,
                estimated_latency_ms: estimated_latency,
                routing_strategy,
            })
        }

        /// Enrutamiento con cadena de fallback explícita
        ///
        /// Si el modelo seleccionado falla, intenta los fallbacks en orden
        /// hasta `max_fallbacks` intentos
        ///
        /// # Errors
        /// Retorna `RoutingError::AllFallbacksExhausted` si todos los fallbacks fallan
        pub fn route_with_fallback(
            &mut self,
            request: RoutingRequest,
            max_fallbacks: usize,
        ) -> Result<RoutingResult, RoutingError> {
            let task = request.task.clone();

            // Intentar enrutamiento directo
            match self.route(request.clone()) {
                Ok(result) => {
                    if max_fallbacks > 0 {
                        let mut extended_result = result.clone();

                        // Agregar fallbacks adicionales si hay candidatos no incluidos
                        if extended_result.fallback_chain.len() < max_fallbacks {
                            let candidates = self.registry.find_by_task(&task);
                            let selected_id = extended_result.selected_model.clone();
                            let extra_fallbacks: Vec<String> = candidates
                                .iter()
                                .filter(|m| {
                                    m.model_id != selected_id
                                        && !extended_result.fallback_chain.contains(&m.model_id)
                                })
                                .map(|m| m.model_id.clone())
                                .take(
                                    max_fallbacks
                                        .saturating_sub(extended_result.fallback_chain.len()),
                                )
                                .collect();
                            extended_result.fallback_chain.extend(extra_fallbacks);
                        }

                        // Si hay una cadena de fallback no vacía, usar estrategia Fallback
                        if !extended_result.fallback_chain.is_empty() {
                            extended_result.routing_strategy = RoutingStrategy::Fallback;
                        }

                        Ok(extended_result)
                    } else {
                        Ok(result)
                    }
                }
                Err(RoutingError::BudgetExceeded { .. }) => {
                    // Intentar con fallbacks de menor rendimiento
                    let candidates = self.registry.find_by_task(&task);
                    let mut attempts = 0;

                    for model in &candidates {
                        if attempts >= max_fallbacks {
                            break;
                        }
                        attempts += 1;

                        if model.latency_p50_ms <= request.latency_budget_ms
                            && model.memory_footprint_mb <= request.memory_budget_mb
                        {
                            let mut fallback_chain: Vec<String> = candidates
                                .iter()
                                .filter(|m| m.model_id != model.model_id)
                                .map(|m| m.model_id.clone())
                                .collect();
                            fallback_chain.sort();

                            let schema_translation = if model.schema_version != request.input_schema
                            {
                                Some(SchemaTranslation::new(
                                    request.input_schema.clone(),
                                    model.schema_version.clone(),
                                    format!(
                                        "Fallback transform from {} to {}",
                                        request.input_schema, model.schema_version
                                    ),
                                ))
                            } else {
                                None
                            };

                            self.stats.total_requests += 1;
                            self.stats.fallback_routes += 1;

                            return Ok(RoutingResult {
                                selected_model: model.model_id.clone(),
                                fallback_chain,
                                schema_translation,
                                estimated_latency_ms: model.latency_p50_ms,
                                routing_strategy: RoutingStrategy::Fallback,
                            });
                        }
                    }

                    Err(RoutingError::AllFallbacksExhausted {
                        task,
                        count: attempts,
                    })
                }
                Err(e) => Err(e),
            }
        }

        /// Actualizar métricas en tiempo de ejecución de un modelo
        ///
        /// Retorna `true` si el modelo fue encontrado en el registro
        pub fn update_model_status(
            &mut self,
            model_id: &str,
            latency_ms: f64,
            memory_mb: usize,
        ) -> bool {
            if self
                .registry
                .get_all()
                .iter()
                .any(|m| m.model_id == model_id)
            {
                self.runtime_status
                    .entry(model_id.to_string())
                    .and_modify(|s| {
                        s.latency_ms = latency_ms;
                        s.memory_mb = memory_mb;
                    })
                    .or_insert(RuntimeStatus {
                        latency_ms,
                        memory_mb,
                        request_count: 0,
                    });
                debug!(
                    "Updated runtime status for model '{}': latency={}ms, memory={}mb",
                    model_id, latency_ms, memory_mb
                );
                true
            } else {
                debug!("Model '{}' not found for status update", model_id);
                false
            }
        }

        /// Obtener estadísticas de enrutamiento
        pub fn get_routing_stats(&self) -> RoutingStats {
            self.stats.clone()
        }

        /// Simular enrutamiento sin actualizar estadísticas
        ///
        /// Retorna una lista de modelos candidatos en orden de preferencia
        pub fn simulate_routing(&self, request: RoutingRequest) -> Vec<String> {
            let candidates = self.registry.find_by_task(&request.task);

            let within_budget: Vec<&ModelCapability> = candidates
                .iter()
                .filter(|m| {
                    let runtime_latency = self
                        .runtime_status
                        .get(&m.model_id)
                        .map(|s| s.latency_ms)
                        .unwrap_or(m.latency_p50_ms);

                    let runtime_memory = self
                        .runtime_status
                        .get(&m.model_id)
                        .map(|s| s.memory_mb)
                        .unwrap_or(m.memory_footprint_mb);

                    runtime_latency <= request.latency_budget_ms
                        && runtime_memory <= request.memory_budget_mb
                })
                .copied()
                .collect();

            let mut result: Vec<String> =
                within_budget.iter().map(|m| m.model_id.clone()).collect();
            result.sort();

            debug!(
                "Simulated routing for task '{}': {} candidates within budget",
                request.task,
                result.len()
            );

            result
        }
    }

    impl Default for CrossModelRouter {
        fn default() -> Self {
            Self::with_defaults()
        }
    }

    // ============================================================================
    // Unit Tests
    // ============================================================================

    #[cfg(test)]
    mod tests {
        use super::*;

        fn sample_capability(
            id: &str,
            name: &str,
            tasks: &[&str],
            schema: &str,
            latency: f64,
            memory: usize,
        ) -> ModelCapability {
            ModelCapability::new(
                id.to_string(),
                name.to_string(),
                schema.to_string(),
                tasks.iter().map(|s| s.to_string()).collect(),
                4096,
                22528,
                32,
                latency,
                latency * 2.5,
                memory,
            )
        }

        fn setup_router() -> CrossModelRouter {
            let mut registry = CapabilityRegistry::new();
            registry
                .register(sample_capability(
                    "m1",
                    "qwen-scope-7b",
                    &["sae_forward", "embedding"],
                    "1.0.0",
                    10.0,
                    256,
                ))
                .unwrap();
            registry
                .register(sample_capability(
                    "m2",
                    "llama-3-8b",
                    &["sae_forward", "feature_extraction"],
                    "1.0.0",
                    15.0,
                    512,
                ))
                .unwrap();
            registry
                .register(sample_capability(
                    "m3",
                    "mistral-7b",
                    &["embedding"],
                    "2.0.0",
                    8.0,
                    128,
                ))
                .unwrap();
            CrossModelRouter::new(registry)
        }

        #[test]
        fn test_direct_routing() {
            let mut router = setup_router();

            let request = RoutingRequest::new(
                "sae_forward".to_string(),
                "1.0.0".to_string(),
                20.0,
                1024,
                RoutingPriority::Normal,
            );

            let result = router.route(request).unwrap();
            assert_eq!(result.selected_model, "m1"); // Lowest latency
            assert_eq!(result.routing_strategy, RoutingStrategy::Direct);
            assert!(result.schema_translation.is_none());
        }

        #[test]
        fn test_fallback_chain_routing() {
            let mut router = setup_router();

            let request = RoutingRequest::new(
                "sae_forward".to_string(),
                "1.0.0".to_string(),
                20.0,
                1024,
                RoutingPriority::Normal,
            );

            let result = router.route_with_fallback(request, 3).unwrap();
            assert_eq!(result.routing_strategy, RoutingStrategy::Fallback);
            assert!(!result.fallback_chain.is_empty());
        }

        #[test]
        fn test_schema_negotiation_and_translation() {
            let mut router = setup_router();

            // Request uses schema 1.0.0, but only m3 supports embedding with schema 2.0.0
            let request = RoutingRequest::new(
                "embedding".to_string(),
                "1.0.0".to_string(),
                20.0,
                512,
                RoutingPriority::Normal,
            );

            let result = router.route(request).unwrap();
            // m3 has lowest latency (8.0ms) but schema 2.0.0
            assert!(result.schema_translation.is_some());
            assert_eq!(result.routing_strategy, RoutingStrategy::SchemaTranslated);
        }

        #[test]
        fn test_load_balancing() {
            let mut router = setup_router();

            // Route multiple requests to same task
            for _ in 0..3 {
                let request = RoutingRequest::new(
                    "sae_forward".to_string(),
                    "1.0.0".to_string(),
                    20.0,
                    1024,
                    RoutingPriority::Normal,
                );
                let _ = router.route(request);
            }

            // Verify request count increased
            let stats = router.get_routing_stats();
            assert_eq!(stats.total_requests, 3);
        }

        #[test]
        fn test_budget_enforcement_latency() {
            let mut router = setup_router();

            let request = RoutingRequest::new(
                "sae_forward".to_string(),
                "1.0.0".to_string(),
                5.0, // Too low for any model
                2048,
                RoutingPriority::Normal,
            );

            let result = router.route(request);
            assert!(matches!(result, Err(RoutingError::BudgetExceeded { .. })));
        }

        #[test]
        fn test_budget_enforcement_memory() {
            let mut router = setup_router();

            let request = RoutingRequest::new(
                "sae_forward".to_string(),
                "1.0.0".to_string(),
                100.0,
                64, // Too low for any model
                RoutingPriority::Normal,
            );

            let result = router.route(request);
            assert!(matches!(result, Err(RoutingError::BudgetExceeded { .. })));
        }

        #[test]
        fn test_priority_based_routing() {
            let mut router = setup_router();

            // Critical priority: no fallbacks allowed
            let request = RoutingRequest::new(
                "sae_forward".to_string(),
                "1.0.0".to_string(),
                20.0,
                1024,
                RoutingPriority::Critical,
            );

            let result = router.route_with_fallback(request.clone(), 5).unwrap();
            // Even though max_fallbacks=5, Critical priority limits to 0
            assert_eq!(result.selected_model, "m1");
        }

        #[test]
        fn test_no_compatible_model() {
            let mut router = setup_router();

            let request = RoutingRequest::new(
                "nonexistent_task".to_string(),
                "1.0.0".to_string(),
                100.0,
                4096,
                RoutingPriority::Normal,
            );

            let result = router.route(request);
            assert!(matches!(
                result,
                Err(RoutingError::NoCompatibleModel { .. })
            ));
        }

        #[test]
        fn test_update_model_status() {
            let mut router = setup_router();

            assert!(router.update_model_status("m1", 5.0, 200));
            assert!(!router.update_model_status("nonexistent", 5.0, 200));

            // After update, m1 should have lower latency
            let request = RoutingRequest::new(
                "sae_forward".to_string(),
                "1.0.0".to_string(),
                20.0,
                1024,
                RoutingPriority::Normal,
            );

            let result = router.route(request).unwrap();
            assert_eq!(result.selected_model, "m1");
            assert!((result.estimated_latency_ms - 5.0).abs() < f64::EPSILON);
        }

        #[test]
        fn test_routing_stats() {
            let mut router = setup_router();

            let request = RoutingRequest::new(
                "sae_forward".to_string(),
                "1.0.0".to_string(),
                20.0,
                1024,
                RoutingPriority::Normal,
            );

            router.route(request).unwrap();

            let stats = router.get_routing_stats();
            assert_eq!(stats.total_requests, 1);
            assert!(stats.avg_latency_ms > 0.0);
        }

        #[test]
        fn test_simulate_routing() {
            let router = setup_router();

            let request = RoutingRequest::new(
                "sae_forward".to_string(),
                "1.0.0".to_string(),
                20.0,
                1024,
                RoutingPriority::Normal,
            );

            let candidates = router.simulate_routing(request);
            assert_eq!(candidates.len(), 2); // m1 and m2
            assert!(candidates.contains(&"m1".to_string()));
            assert!(candidates.contains(&"m2".to_string()));

            // Stats should not change after simulation
            let stats = router.get_routing_stats();
            assert_eq!(stats.total_requests, 0);
        }

        #[test]
        fn test_fallback_exhaustion() {
            let mut router = setup_router();

            let request = RoutingRequest::new(
                "sae_forward".to_string(),
                "1.0.0".to_string(),
                1.0, // Impossible budget
                32,
                RoutingPriority::Normal,
            );

            let result = router.route_with_fallback(request, 5);
            assert!(matches!(
                result,
                Err(RoutingError::AllFallbacksExhausted { .. })
            ));
        }

        #[test]
        fn test_with_defaults() {
            let router = CrossModelRouter::with_defaults();
            let stats = router.get_routing_stats();
            assert_eq!(stats.total_requests, 0);
        }
    }
}

#[cfg(feature = "v1.1-sprint1")]
pub use impl_::*;
