//! Semantic Map - Mapeo feature→concepto (Qwen-Scope metadata)
//!
//! Fase 2: Infraestructura básica con conceptos placeholder.
//! Fase 3: Integración con Qwen-Scope + aprendizaje desde feedback humano.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

/// Metadata de Qwen-Scope para carga de conceptos
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QwenScopeMetadata {
    pub concepts: Vec<QwenScopeConcept>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QwenScopeConcept {
    pub name: String,
    pub description: String,
    pub category: String,
    pub feature_indices: Vec<u32>,
    pub relevant_layers: Vec<u32>,
    pub activation_threshold: f32,
    pub importance: f32,
    pub metadata: Option<serde_json::Value>,
}

// ============================================================================
// Concept Metadata
// ============================================================================

/// Concepto semántico asociado a una feature SAE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticConcept {
    /// ID único del concepto
    pub concept_id: String,
    /// Nombre del concepto (ej: "self_reference", "logical_negation")
    pub name: String,
    /// Descripción del concepto
    pub description: String,
    /// Categoría (emotion, logic, factuality, style, etc.)
    pub category: ConceptCategory,
    /// Feature indices asociados (neuron indices del SAE)
    pub feature_indices: Vec<u32>,
    /// Capas SAE donde este concepto es relevante
    pub relevant_layers: Vec<u32>,
    /// Umbral de activación para considerar el concepto "activo"
    pub activation_threshold: f32,
    /// Importancia relativa del concepto (0.0 - 1.0)
    pub importance: f32,
    /// Metadata adicional (JSON)
    pub metadata: Option<serde_json::Value>,
}

/// Categoría de concepto
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConceptCategory {
    /// Lógica y razonamiento
    Logic,
    /// Emociones y tono
    Emotion,
    /// Facticidad y veracidad
    Factuality,
    /// Estilo y formato
    Style,
    /// Seguridad y ética
    Safety,
    /// Repetición y degeneración
    Repetition,
    /// Contradicción y conflicto
    Contradiction,
    /// Otro
    Other(String),
}

impl std::fmt::Display for ConceptCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConceptCategory::Logic => write!(f, "logic"),
            ConceptCategory::Emotion => write!(f, "emotion"),
            ConceptCategory::Factuality => write!(f, "factuality"),
            ConceptCategory::Style => write!(f, "style"),
            ConceptCategory::Safety => write!(f, "safety"),
            ConceptCategory::Repetition => write!(f, "repetition"),
            ConceptCategory::Contradiction => write!(f, "contradiction"),
            ConceptCategory::Other(s) => write!(f, "{}", s),
        }
    }
}

// ============================================================================
// Semantic Map
// ============================================================================

/// Mapa semántico feature→concepto
pub struct SemanticMap {
    /// Conceptos por ID
    concepts: HashMap<String, SemanticConcept>,
    /// Index inverso: feature_index → concept_ids
    feature_to_concepts: HashMap<u32, Vec<String>>,
    /// Index por categoría
    category_to_concepts: HashMap<ConceptCategory, Vec<String>>,
    /// Total de conceptos cargados
    total_concepts: usize,
}

impl SemanticMap {
    /// Crear nuevo mapa semántico con conceptos placeholder
    pub fn new() -> Self {
        let mut map = Self {
            concepts: HashMap::new(),
            feature_to_concepts: HashMap::new(),
            category_to_concepts: HashMap::new(),
            total_concepts: 0,
        };

        // Cargar conceptos placeholder basados en investigación SAE
        map.load_default_concepts();

        debug!(
            "SemanticMap inicializado con {} conceptos",
            map.total_concepts
        );
        map
    }

    /// Cargar conceptos placeholder por defecto
    fn load_default_concepts(&mut self) {
        let defaults = vec![
            SemanticConcept {
                concept_id: "self_ref_001".to_string(),
                name: "self_reference".to_string(),
                description: "Referencia al propio modelo o identidad de IA".to_string(),
                category: ConceptCategory::Logic,
                feature_indices: vec![100, 101, 102],
                relevant_layers: vec![10, 14, 20],
                activation_threshold: 0.7,
                importance: 0.8,
                metadata: None,
            },
            SemanticConcept {
                concept_id: "contradict_001".to_string(),
                name: "logical_contradiction".to_string(),
                description: "Detección de contradicción lógica en el texto".to_string(),
                category: ConceptCategory::Contradiction,
                feature_indices: vec![200, 201, 202, 203],
                relevant_layers: vec![8, 12, 16],
                activation_threshold: 0.6,
                importance: 0.9,
                metadata: None,
            },
            SemanticConcept {
                concept_id: "repeat_001".to_string(),
                name: "repetition_pattern".to_string(),
                description: "Patrón de repetición o degeneración del texto".to_string(),
                category: ConceptCategory::Repetition,
                feature_indices: vec![300, 301, 302],
                relevant_layers: vec![6, 10, 14],
                activation_threshold: 0.5,
                importance: 0.7,
                metadata: None,
            },
            SemanticConcept {
                concept_id: "safety_001".to_string(),
                name: "safety_concern".to_string(),
                description: "Contenido potencialmente inseguro o dañino".to_string(),
                category: ConceptCategory::Safety,
                feature_indices: vec![400, 401, 402, 403, 404],
                relevant_layers: vec![4, 8, 12, 16],
                activation_threshold: 0.65,
                importance: 0.95,
                metadata: None,
            },
            SemanticConcept {
                concept_id: "emotion_001".to_string(),
                name: "positive_sentiment".to_string(),
                description: "Sentimiento positivo o optimista en el texto".to_string(),
                category: ConceptCategory::Emotion,
                feature_indices: vec![500, 501],
                relevant_layers: vec![10, 14, 18],
                activation_threshold: 0.6,
                importance: 0.6,
                metadata: None,
            },
            SemanticConcept {
                concept_id: "emotion_002".to_string(),
                name: "negative_sentiment".to_string(),
                description: "Sentimiento negativo o pesimista en el texto".to_string(),
                category: ConceptCategory::Emotion,
                feature_indices: vec![510, 511],
                relevant_layers: vec![10, 14, 18],
                activation_threshold: 0.6,
                importance: 0.6,
                metadata: None,
            },
            SemanticConcept {
                concept_id: "fact_001".to_string(),
                name: "factual_claim".to_string(),
                description: "Afirmación factual que requiere verificación".to_string(),
                category: ConceptCategory::Factuality,
                feature_indices: vec![600, 601, 602],
                relevant_layers: vec![12, 16, 20],
                activation_threshold: 0.7,
                importance: 0.85,
                metadata: None,
            },
            SemanticConcept {
                concept_id: "style_001".to_string(),
                name: "formal_tone".to_string(),
                description: "Tono formal o académico en el texto".to_string(),
                category: ConceptCategory::Style,
                feature_indices: vec![700, 701],
                relevant_layers: vec![8, 12],
                activation_threshold: 0.5,
                importance: 0.4,
                metadata: None,
            },
        ];

        for concept in defaults {
            self.add_concept(concept);
        }
    }

    /// Agregar concepto al mapa
    pub fn add_concept(&mut self, concept: SemanticConcept) {
        let concept_id = concept.concept_id.clone();

        // Index por feature
        for &feature_idx in &concept.feature_indices {
            self.feature_to_concepts
                .entry(feature_idx)
                .or_default()
                .push(concept_id.clone());
        }

        // Index por categoría
        self.category_to_concepts
            .entry(concept.category.clone())
            .or_default()
            .push(concept_id.clone());

        self.concepts.insert(concept_id, concept);
        self.total_concepts += 1;
    }

    /// Buscar conceptos por feature index
    pub fn lookup_by_feature(&self, feature_index: u32) -> Vec<&SemanticConcept> {
        let concept_ids = self.feature_to_concepts.get(&feature_index);
        match concept_ids {
            Some(ids) => ids
                .iter()
                .filter_map(|id| self.concepts.get(id))
                .collect(),
            None => vec![],
        }
    }

    /// Buscar conceptos por categoría
    pub fn lookup_by_category(&self, category: &ConceptCategory) -> Vec<&SemanticConcept> {
        let concept_ids = self.category_to_concepts.get(category);
        match concept_ids {
            Some(ids) => ids
                .iter()
                .filter_map(|id| self.concepts.get(id))
                .collect(),
            None => vec![],
        }
    }

    /// Buscar concepto por ID
    pub fn get_concept(&self, concept_id: &str) -> Option<&SemanticConcept> {
        self.concepts.get(concept_id)
    }

    /// Obtener todos los conceptos activos dado un set de features activas
    pub fn get_active_concepts(
        &self,
        active_features: &[(u32, f32)], // (feature_index, activation_value)
    ) -> Vec<ActiveConcept> {
        let mut active = Vec::new();

        for &(feature_idx, activation) in active_features {
            let concepts = self.lookup_by_feature(feature_idx);
            for concept in concepts {
                if activation >= concept.activation_threshold {
                    active.push(ActiveConcept {
                        concept_id: concept.concept_id.clone(),
                        name: concept.name.clone(),
                        category: concept.category.clone(),
                        activation,
                        importance: concept.importance,
                        relevance: activation * concept.importance,
                    });
                }
            }
        }

        // Ordenar por relevancia
        active.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());
        active
    }

    /// Generar descripción semántica de un batch de features
    pub fn describe_features(
        &self,
        active_features: &[(u32, f32)],
        max_concepts: usize,
    ) -> String {
        let active = self.get_active_concepts(active_features);
        let limited = active.iter().take(max_concepts);

        let descriptions: Vec<String> = limited
            .map(|ac| {
                format!(
                    "[{}] ({:.2}) - {}",
                    ac.name, ac.activation, ac.category
                )
            })
            .collect();

        if descriptions.is_empty() {
            "No concepts identified".to_string()
        } else {
            descriptions.join(", ")
        }
    }

    /// Obtener total de conceptos
    pub fn total_concepts(&self) -> usize {
        self.total_concepts
    }

    /// Obtener categorías disponibles
    pub fn available_categories(&self) -> Vec<&ConceptCategory> {
        self.category_to_concepts.keys().collect()
    }

    /// Cargar conceptos desde metadata de Qwen-Scope
    ///
    /// Lee un archivo JSON con metadata de Qwen-Scope y actualiza el mapa semántico.
    /// Formato esperado:
    /// ```json
    /// {
    ///   "concepts": [
    ///     {
    ///       "name": "concept_name",
    ///       "description": "Description",
    ///       "category": "logic",
    ///       "feature_indices": [100, 101],
    ///       "relevant_layers": [10, 14],
    ///       "activation_threshold": 0.7,
    ///       "importance": 0.8
    ///     }
    ///   ]
    /// }
    /// ```
    pub fn load_from_qwen_scope(&mut self, path: &str) -> anyhow::Result<()> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read Qwen-Scope metadata '{}': {}", path, e))?;

        let metadata: QwenScopeMetadata = serde_json::from_str(&data)
            .map_err(|e| anyhow::anyhow!("Failed to parse Qwen-Scope metadata: {}", e))?;

        let mut loaded = 0;
        for concept_data in metadata.concepts {
            let category = match concept_data.category.to_lowercase().as_str() {
                "logic" => ConceptCategory::Logic,
                "emotion" => ConceptCategory::Emotion,
                "factuality" => ConceptCategory::Factuality,
                "style" => ConceptCategory::Style,
                "safety" => ConceptCategory::Safety,
                "repetition" => ConceptCategory::Repetition,
                "contradiction" => ConceptCategory::Contradiction,
                other => ConceptCategory::Other(other.to_string()),
            };

            let concept_id = format!("qwen_{}", concept_data.name.replace(' ', "_"));

            let concept = SemanticConcept {
                concept_id,
                name: concept_data.name,
                description: concept_data.description,
                category,
                feature_indices: concept_data.feature_indices,
                relevant_layers: concept_data.relevant_layers,
                activation_threshold: concept_data.activation_threshold,
                importance: concept_data.importance,
                metadata: concept_data.metadata,
            };

            self.add_concept(concept);
            loaded += 1;
        }

        tracing::info!(
            "Loaded {} concepts from Qwen-Scope metadata (total: {})",
            loaded,
            self.total_concepts
        );
        Ok(())
    }

    /// Aprender nuevos conceptos desde feedback humano
    ///
    /// Crea un nuevo concepto semántico basado en features y label proporcionados.
    /// Validación: verifica que las features existen y que el label es único.
    pub fn learn_concept(
        &mut self,
        features: &[u32],
        label: &str,
        description: Option<&str>,
        category: Option<ConceptCategory>,
    ) -> anyhow::Result<String> {
        if features.is_empty() {
            return Err(anyhow::anyhow!("Feature list cannot be empty"));
        }

        if label.is_empty() {
            return Err(anyhow::anyhow!("Label cannot be empty"));
        }

        // Verifica que el label no exista ya
        for concept in self.concepts.values() {
            if concept.name == label {
                return Err(anyhow::anyhow!("Concept '{}' already exists", label));
            }
        }

        // Genera concept_id único
        let concept_id = format!("learned_{}", label.replace(' ', "_"));

        let description = description
            .unwrap_or("Concept learned from human feedback")
            .to_string();

        let category = category.unwrap_or(ConceptCategory::Other("learned".to_string()));

        let concept = SemanticConcept {
            concept_id: concept_id.clone(),
            name: label.to_string(),
            description,
            category,
            feature_indices: features.to_vec(),
            relevant_layers: vec![], // Se determinará con más datos
            activation_threshold: 0.6, // Umbral por defecto
            importance: 0.5, // Importancia moderada inicial
            metadata: Some(serde_json::json!({
                "source": "human_feedback",
                "learned_at": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            })),
        };

        self.add_concept(concept);

        tracing::info!(
            "Learned new concept '{}' from {} features (total: {})",
            label,
            features.len(),
            self.total_concepts
        );

        Ok(concept_id)
    }

    /// Actualiza un concepto existente con nueva información
    pub fn update_concept(&mut self, concept_id: &str, new_name: Option<&str>) -> anyhow::Result<()> {
        let concept = self.concepts.get_mut(concept_id)
            .ok_or_else(|| anyhow::anyhow!("Concept '{}' not found", concept_id))?;

        if let Some(name) = new_name {
            concept.name = name.to_string();
        }

        tracing::debug!("Updated concept '{}'", concept_id);
        Ok(())
    }
}

/// Concepto activo con su nivel de activación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveConcept {
    pub concept_id: String,
    pub name: String,
    pub category: ConceptCategory,
    pub activation: f32,
    pub importance: f32,
    pub relevance: f32,
}

impl Default for SemanticMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_map_initialization() {
        let map = SemanticMap::new();
        assert!(map.total_concepts() > 0);
    }

    #[test]
    fn test_lookup_by_feature() {
        let map = SemanticMap::new();
        let concepts = map.lookup_by_feature(100);
        assert!(!concepts.is_empty());
        assert_eq!(concepts[0].name, "self_reference");
    }

    #[test]
    fn test_active_concepts() {
        let map = SemanticMap::new();
        let features = vec![(100u32, 0.9f32), (200u32, 0.8f32)];
        let active = map.get_active_concepts(&features);
        assert!(!active.is_empty());
    }

    #[test]
    fn test_describe_features() {
        let map = SemanticMap::new();
        let features = vec![(100u32, 0.95f32)];
        let description = map.describe_features(&features, 5);
        assert!(description.contains("self_reference"));
    }
}
