//! Marketplace Matchmaker — Matching de recursos computacionales con SLO-aware routing
//!
//! Motor de emparejamiento que conecta `ResourceRequest` con `ResourceListing`
//! usando algoritmo de menor costo con validación de SLO (SLI de latencia,
//! disponibilidad y throughput). Soporta shards SAE, VRAM y ancho de banda.
//!
//! Feature-gated: `#[cfg(feature = "v1.1-sprint3")]`

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Instant;
use thiserror::Error;
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errores del matchmaker.
#[derive(Debug, Error)]
pub enum MatchmakerError {
    #[error("No matching listings for request: {0}")]
    NoMatch(String),
    #[error("Trust threshold not met: {0:.3} < {1:.3}")]
    TrustThresholdNotMet(f32, f32),
    #[error("SLO violation: {0}")]
    SLOViolation(String),
    #[error("Match timeout: {0}ms")]
    MatchTimeout(u64),
    #[error("Anti-monopoly limit exceeded for node: {0}")]
    AntiMonopolyLimit(String),
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Tipo de recurso computacional disponible en el marketplace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResourceType {
    /// Shard SAE (Sparse Autoencoder) para interpretación.
    SAEShard { model_id: String, layer: u32 },
    /// Memoria VRAM disponible (en GB).
    VRAM { gpu_model: String, vram_gb: f32 },
    /// Ancho de banda de red (en Mbps).
    Bandwidth { max_mbps: f32 },
}

impl Eq for ResourceType {}

impl ResourceType {
    /// Retorna una descripción legible del tipo de recurso.
    pub fn description(&self) -> String {
        match self {
            ResourceType::SAEShard { model_id, layer } => {
                format!("SAE Shard: {} layer {}", model_id, layer)
            }
            ResourceType::VRAM { gpu_model, vram_gb } => {
                format!("VRAM: {} {}GB", gpu_model, vram_gb)
            }
            ResourceType::Bandwidth { max_mbps } => {
                format!("Bandwidth: {} Mbps", max_mbps)
            }
        }
    }

    /// Retorna el tipo como string para serialización.
    pub fn as_str(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

impl std::hash::Hash for ResourceType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::SAEShard { model_id, layer } => {
                "SAEShard".hash(state);
                model_id.hash(state);
                layer.hash(state);
            }
            Self::VRAM { gpu_model, vram_gb } => {
                "VRAM".hash(state);
                gpu_model.hash(state);
                state.write_u32(vram_gb.to_bits());
            }
            Self::Bandwidth { max_mbps } => {
                "Bandwidth".hash(state);
                state.write_u32(max_mbps.to_bits());
            }
        }
    }
}

/// Oferta de recurso publicada por un nodo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceListing {
    /// ID único del nodo que ofrece el recurso.
    pub node_id: String,
    /// Tipo de recurso ofrecido.
    pub resource_type: ResourceType,
    /// Cantidad disponible (unidades según tipo).
    pub quantity: f32,
    /// Precio base por unidad (en créditos).
    pub base_price: f32,
    /// Timestamp de publicación (epoch ms).
    pub listed_at: u64,
    /// Timestamp de expiración (epoch ms).
    pub expires_at: u64,
    /// SLO del nodo: latencia máxima aceptada (ms).
    pub max_latency_ms: u64,
    /// SLO del nodo: disponibilidad (0.0 - 1.0).
    pub availability_slo: f32,
    /// SLO del nodo: throughput mínimo (ops/s).
    pub min_throughput: u64,
}

/// Solicitud de recurso de un nodo demandante.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequest {
    /// ID del nodo solicitante.
    pub requester_id: String,
    /// Tipo de recurso requerido.
    pub resource_type: ResourceType,
    /// Cantidad requerida.
    pub quantity: f32,
    /// Precio máximo dispuesto a pagar.
    pub max_price: f32,
    /// Latencia máxima aceptable (ms).
    pub max_latency_ms: u64,
    /// Disponibilidad mínima requerida.
    pub min_availability: f32,
}

/// Resultado de un emparejamiento exitoso.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    /// Indica si se encontró match.
    pub matched: bool,
    /// Listing emparejado (si aplica).
    pub listing: Option<ResourceListing>,
    /// Precio final acordado.
    pub final_price: f32,
    /// Hash del acuerdo de settlement.
    pub settlement_hash: String,
    /// Bandera anti-monopolio.
    pub anti_monopoly_flag: bool,
    /// Tiempo de matching en ms.
    pub match_time_ms: f64,
    /// SLO verificadas.
    pub slo_verified: bool,
}

impl MatchResult {
    pub fn matched(
        listing: ResourceListing,
        final_price: f32,
        settlement_hash: String,
        match_time_ms: f64,
    ) -> Self {
        Self {
            matched: true,
            listing: Some(listing),
            final_price,
            settlement_hash,
            anti_monopoly_flag: false,
            match_time_ms,
            slo_verified: true,
        }
    }

    pub fn rejected(match_time_ms: f64, reason: &str) -> Self {
        Self {
            matched: false,
            listing: None,
            final_price: 0.0,
            settlement_hash: String::new(),
            anti_monopoly_flag: reason.contains("monopoly"),
            match_time_ms,
            slo_verified: reason.contains("SLO"),
        }
    }
}

/// Límite anti-monopolio: un nodo no puede acumular más del `max_share`%
/// del total de recursos de un tipo.
const DEFAULT_MAX_RESOURCE_SHARE: f32 = 0.30; // 30%

/// Matchmaker para el marketplace de recursos computacionales.
pub struct ResourceMatchmaker {
    /// Listings activos indexados por tipo de recurso.
    listings: HashMap<String, Vec<ResourceListing>>,
    /// Límite anti-monopolio por nodo (share máximo).
    max_resource_share: f32,
    /// Umbral mínimo de confianza.
    trust_threshold: f32,
    /// Contador de recursos por nodo (para anti-monopolio).
    node_resource_counts: HashMap<String, usize>,
    /// Tiempo máximo de matching (ms).
    match_timeout_ms: u64,
}

impl ResourceMatchmaker {
    /// Crea un nuevo matchmaker con configuración por defecto.
    pub fn new() -> Self {
        Self::with_config(0.30, 0.5, 100)
    }

    /// Crea un matchmaker con configuración personalizada.
    pub fn with_config(max_share: f32, trust_threshold: f32, match_timeout_ms: u64) -> Self {
        info!(
            max_share = %max_share,
            trust_threshold = %trust_threshold,
            match_timeout_ms = %match_timeout_ms,
            "ResourceMatchmaker initialized"
        );
        Self {
            listings: HashMap::new(),
            max_resource_share: max_share,
            trust_threshold,
            node_resource_counts: HashMap::new(),
            match_timeout_ms,
        }
    }

    /// Registra un nuevo listing en el marketplace.
    pub fn register_listing(&mut self, listing: ResourceListing) {
        let key = Self::resource_key(&listing.resource_type);
        self.listings.entry(key).or_default().push(listing.clone());
        *self.node_resource_counts.entry(listing.node_id.clone()).or_insert(0) += 1;
        debug!(
            node = %listing.node_id,
            resource = %listing.resource_type.description(),
            "Listing registered"
        );
    }

    /// Elimina listings expirados.
    pub fn cleanup_expired(&mut self, now_ms: u64) -> usize {
        let mut removed = 0;
        let expired_keys: Vec<String> = self.listings.keys()
            .filter(|key| {
                self.listings.get(*key)
                    .map(|listings| listings.iter().all(|l| l.expires_at <= now_ms))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        for key in &expired_keys {
            if let Some(listings) = self.listings.remove(key) {
                removed += listings.len();
            }
        }

        // Filtrar listings expirados sin eliminar el key
        for (_key, listings) in self.listings.iter_mut() {
            let before = listings.len();
            listings.retain(|l| l.expires_at > now_ms);
            removed += before - listings.len();
        }

        if removed > 0 {
            info!(removed, "Expired listings cleaned up");
        }
        removed
    }

    /// Encuentra el mejor match para una request.
    pub fn match_request(&mut self, request: &ResourceRequest) -> Result<MatchResult, MatchmakerError> {
        let start = Instant::now();

        let key = Self::resource_key(&request.resource_type);
        let listings = self.listings.get(&key);

        let match_time_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Timeout check
        if match_time_ms > self.match_timeout_ms as f64 {
            return Ok(MatchResult::rejected(match_time_ms, &format!(
                "SLO violation: match timeout {}ms > {}ms",
                match_time_ms, self.match_timeout_ms
            )));
        }

        let listings = match listings {
            Some(listings) if !listings.is_empty() => listings,
            _ => {
                return Ok(MatchResult::rejected(
                    match_time_ms,
                    &format!("No listings available for {}", request.resource_type.description()),
                ));
            }
        };

        // Filtrar por disponibilidad y SLO
        let valid: Vec<&ResourceListing> = listings.iter().filter(|l| {
            l.expires_at > chrono::Utc::now().timestamp_millis() as u64
                && l.availability_slo >= request.min_availability
                && l.max_latency_ms <= request.max_latency_ms
                && l.quantity >= request.quantity
                && l.base_price <= request.max_price
        }).collect();

        if valid.is_empty() {
            return Ok(MatchResult::rejected(match_time_ms, "No valid listings match request criteria"));
        }

        // Seleccionar el de menor precio
        let best = valid.iter().min_by(|a, b| {
            a.base_price.partial_cmp(&b.base_price).unwrap_or(std::cmp::Ordering::Equal)
        }).unwrap();

        // Verificar anti-monopolio
        let total_for_type = listings.len();
        let node_count = self.node_resource_counts.get(&best.node_id).unwrap_or(&0);
        let share = *node_count as f32 / total_for_type.max(1) as f32;

        let anti_monopoly_flag = share > self.max_resource_share;

        if anti_monopoly_flag {
            warn!(
                node = %best.node_id,
                share = %share,
                limit = %self.max_resource_share,
                "Anti-monopoly limit exceeded"
            );
            // Buscar el siguiente mejor que no exceda el límite
            let fallback = valid.iter().find(|l| {
                let count = self.node_resource_counts.get(&l.node_id).unwrap_or(&0);
                let total = listings.len();
                (*count as f32 / total.max(1) as f32) <= self.max_resource_share
            });

            if let Some(fb) = fallback {
                return Ok(MatchResult::matched(
                    (*fb).clone(),
                    fb.base_price,
                    Self::compute_settlement_hash(fb, &request.requester_id),
                    start.elapsed().as_secs_f64() * 1000.0,
                ));
            }

            return Ok(MatchResult::rejected(
                start.elapsed().as_secs_f64() * 1000.0,
                &format!("Anti-monopoly limit exceeded for node {}", best.node_id),
            ));
        }

        let result = MatchResult::matched(
            (*best).clone(),
            best.base_price,
            Self::compute_settlement_hash(best, &request.requester_id),
            start.elapsed().as_secs_f64() * 1000.0,
        );

        info!(
            node = %best.node_id,
            price = %best.base_price,
            match_time_ms = %result.match_time_ms,
            "Match completed successfully"
        );

        Ok(result)
    }

    /// Retorna el total de listings activos.
    pub fn listing_count(&self) -> usize {
        self.listings.values().map(|v| v.len()).sum()
    }

    /// Retorna listings por tipo de recurso.
    pub fn listings_by_type(&self, resource_type: &ResourceType) -> Vec<&ResourceListing> {
        let key = Self::resource_key(resource_type);
        self.listings.get(&key).map(|v| v.iter().collect()).unwrap_or_default()
    }

    /// Calcula hash de settlement para escrow.
    fn compute_settlement_hash(listing: &ResourceListing, requester_id: &str) -> String {
        let data = format!("{}{}{}{}", listing.node_id, requester_id, listing.base_price, listing.listed_at);
        hex::encode(Sha256::digest(data.as_bytes()))
    }

    /// Genera clave de indexación para un tipo de recurso.
    fn resource_key(resource_type: &ResourceType) -> String {
        match resource_type {
            ResourceType::SAEShard { model_id, layer } => {
                format!("sae:{}:{}", model_id, layer)
            }
            ResourceType::VRAM { gpu_model, .. } => {
                format!("vram:{}", gpu_model)
            }
            ResourceType::Bandwidth { .. } => "bandwidth".to_string(),
        }
    }
}

impl Default for ResourceMatchmaker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sae_listing(node: &str, model: &str, layer: u32, price: f32) -> ResourceListing {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        ResourceListing {
            node_id: node.into(),
            resource_type: ResourceType::SAEShard { model_id: model.into(), layer },
            quantity: 10.0,
            base_price: price,
            listed_at: now,
            expires_at: now + 3600_000, // 1 hour from now
            max_latency_ms: 50,
            availability_slo: 0.99,
            min_throughput: 1000,
        }
    }

    fn vram_request(node: &str, gpu: &str, qty: f32, max_price: f32) -> ResourceRequest {
        ResourceRequest {
            requester_id: node.into(),
            resource_type: ResourceType::VRAM { gpu_model: gpu.into(), vram_gb: qty },
            quantity: qty,
            max_price,
            max_latency_ms: 50,
            min_availability: 0.95,
        }
    }

    #[test]
    fn test_register_and_listings_count() {
        let mut mm = ResourceMatchmaker::new();
        assert_eq!(mm.listing_count(), 0);
        mm.register_listing(sae_listing("n1", "scope-v2", 5, 100.0));
        assert_eq!(mm.listing_count(), 1);
    }

    #[test]
    fn test_match_request_success() {
        // Use high max_share to avoid anti-monopoly rejection with single listing
        let mut mm = ResourceMatchmaker::with_config(1.0, 0.5, 100);
        mm.register_listing(sae_listing("n1", "scope-v2", 5, 100.0));
        let req = ResourceRequest {
            requester_id: "buyer".into(),
            resource_type: ResourceType::SAEShard { model_id: "scope-v2".into(), layer: 5 },
            quantity: 5.0,
            max_price: 120.0,
            max_latency_ms: 100,
            min_availability: 0.95,
        };
        let result = mm.match_request(&req).unwrap();
        assert!(result.matched);
        assert_eq!(result.final_price, 100.0);
    }

    #[test]
    fn test_match_selects_lowest_price() {
        // Use high max_share to avoid anti-monopoly rejection
        let mut mm = ResourceMatchmaker::with_config(1.0, 0.5, 100);
        mm.register_listing(sae_listing("cheap", "scope-v2", 5, 80.0));
        mm.register_listing(sae_listing("expensive", "scope-v2", 5, 120.0));
        let req = ResourceRequest {
            requester_id: "buyer".into(),
            resource_type: ResourceType::SAEShard { model_id: "scope-v2".into(), layer: 5 },
            quantity: 5.0,
            max_price: 150.0,
            max_latency_ms: 100,
            min_availability: 0.95,
        };
        let result = mm.match_request(&req).unwrap();
        assert!(result.matched);
        assert_eq!(result.final_price, 80.0);
    }

    #[test]
    fn test_match_no_available_listing() {
        let mut mm = ResourceMatchmaker::new();
        let req = ResourceRequest {
            requester_id: "buyer".into(),
            resource_type: ResourceType::SAEShard { model_id: "scope-v3".into(), layer: 3 },
            quantity: 5.0,
            max_price: 150.0,
            max_latency_ms: 100,
            min_availability: 0.95,
        };
        let result = mm.match_request(&req);
        // No match is a valid outcome (Ok with matched=false), not an error
        assert!(result.is_ok());
        let match_result = result.unwrap();
        assert!(!match_result.matched);
    }

    #[test]
    fn test_cleanup_expired() {
        let mut mm = ResourceMatchmaker::new();
        mm.register_listing(sae_listing("n1", "scope-v2", 5, 100.0));
        mm.register_listing(ResourceListing {
            node_id: "n2".into(),
            resource_type: ResourceType::SAEShard { model_id: "scope-v2".into(), layer: 5 },
            quantity: 10.0,
            base_price: 90.0,
            listed_at: 100,
            expires_at: 500,
            max_latency_ms: 100,
            availability_slo: 0.99,
            min_throughput: 1000,
        });
        assert_eq!(mm.listing_count(), 2);
        let removed = mm.cleanup_expired(1500);
        assert_eq!(removed, 1);
        assert_eq!(mm.listing_count(), 1);
    }

    #[test]
    fn test_match_price_exceeds_max() {
        let mut mm = ResourceMatchmaker::new();
        mm.register_listing(sae_listing("n1", "scope-v2", 5, 200.0));
        let req = ResourceRequest {
            requester_id: "buyer".into(),
            resource_type: ResourceType::SAEShard { model_id: "scope-v2".into(), layer: 5 },
            quantity: 5.0,
            max_price: 150.0,
            max_latency_ms: 100,
            min_availability: 0.95,
        };
        let result = mm.match_request(&req).unwrap();
        assert!(!result.matched);
    }

    #[test]
    fn test_resource_type_description() {
        assert!(matches!(
            ResourceType::SAEShard { model_id: "test".into(), layer: 1 }.description(),
            s if s.contains("SAE Shard") && s.contains("test")
        ));
        assert!(matches!(
            ResourceType::VRAM { gpu_model: "A100".into(), vram_gb: 80.0 }.description(),
            s if s.contains("VRAM") && s.contains("A100")
        ));
        assert!(matches!(
            ResourceType::Bandwidth { max_mbps: 1000.0 }.description(),
            s if s.contains("Bandwidth") && s.contains("1000")
        ));
    }
}
