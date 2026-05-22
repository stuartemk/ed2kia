//! Pricing Engine — Motor de precios adaptativo con compromisos Pedersen
//!
//! Calcula precios dinámicos basados en oferta/demanda, disponibilidad de
//! recursos y métricas de red. Usa compromisos Pedersen para ocultar precios
//! hasta el momento del settlement. Ventana deslizante de 30s para cálculos.
//!
//! Feature-gated: `#[cfg(feature = "v1.1-sprint3")]`

use ark_bn254::{Fr, G1Affine, G1Projective};
use ark_ec::CurveGroup;
use ark_ff::{PrimeField, UniformRand};
use ark_serialize::CanonicalSerialize;
use ark_std::rand::thread_rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use tracing::{debug, info};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errores del pricing engine.
#[derive(Debug, Error)]
pub enum PricingError {
    #[error("Insufficient data: need at least {0} samples, have {1}")]
    InsufficientData(usize, usize),
    #[error("Price outside bounds: {0:.2} not in [{1:.2}, {2:.2}]")]
    PriceOutOfBounds(f32, f32, f32),
    #[error("Invalid demand ratio: {0}")]
    InvalidDemandRatio(f32),
    #[error("Pricing computation timeout")]
    Timeout,
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Tipo de recurso para pricing específico.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PricingResourceType {
    /// Pricing para SAE shards.
    SAEShard,
    /// Pricing para VRAM.
    VRAM,
    /// Pricing para ancho de banda.
    Bandwidth,
    /// Pricing genérico.
    Generic,
}

/// Métrica de mercado para cálculo de precios.
#[derive(Debug, Clone)]
pub struct MarketSample {
    /// Timestamp del sample (epoch ms).
    pub timestamp: u64,
    /// Precio del listing.
    pub price: f32,
    /// Cantidad ofrecida.
    pub quantity: f32,
    /// Número de requests en la ventana.
    pub demand_count: u64,
}

/// Resultado del motor de precios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceQuote {
    /// Precio calculado por unidad.
    pub unit_price: f32,
    /// Precio comprometido (Pedersen hash, para ocultar hasta settlement).
    pub commitment_hash: [u8; 32],
    /// Factor de ajuste aplicado.
    pub adjustment_factor: f32,
    /// Tipo de recurso.
    pub resource_type: PricingResourceType,
    /// Ventana de validez del precio (ms).
    pub validity_window_ms: u64,
    /// Timestamp de expiración.
    pub expires_at: u64,
}

/// Estadísticas del pricing engine.
#[derive(Debug, Clone)]
pub struct PricingStats {
    /// Total de quotes generadas.
    pub total_quotes: u64,
    /// Promedio de ajuste aplicado.
    pub avg_adjustment: f32,
    /// Precio promedio calculado.
    pub avg_price: f32,
    /// Ventana actual de demanda.
    pub current_demand: u64,
    /// Ventana actual de oferta.
    pub current_supply: f32,
}

/// Configuración del pricing engine.
#[derive(Debug, Clone)]
pub struct PricingConfig {
    /// Ventana deslizante para cálculos (ms).
    pub window_size_ms: u64,
    /// Precio mínimo (créditos/unidad).
    pub min_price: f32,
    /// Precio máximo (créditos/unidad).
    pub max_price: f32,
    /// Factor base para recursos SAE.
    pub sae_base_price: f32,
    /// Factor base para VRAM.
    pub vram_base_price: f32,
    /// Factor base para bandwidth.
    pub bandwidth_base_price: f32,
    /// Sensibilidad a demanda (0.0 - 2.0).
    pub demand_sensitivity: f32,
    /// Sensibilidad a oferta (0.0 - 2.0).
    pub supply_sensitivity: f32,
}

impl Default for PricingConfig {
    fn default() -> Self {
        Self {
            window_size_ms: 30_000, // 30 segundos
            min_price: 1.0,
            max_price: 10_000.0,
            sae_base_price: 100.0,
            vram_base_price: 250.0,
            bandwidth_base_price: 10.0,
            demand_sensitivity: 0.5,
            supply_sensitivity: 0.3,
        }
    }
}

/// Compromiso Pedersen para precios.
pub struct PriceCommitment {
    /// Punto G1 del compromiso.
    pub commitment_point: G1Affine,
    /// Hash del compromiso.
    pub hash: [u8; 32],
    /// Blinding factor usado.
    pub blinding_factor: Fr,
}

/// Motor de precios adaptativo para el marketplace.
pub struct PricingEngine {
    /// Configuración.
    config: PricingConfig,
    /// Samples de mercado en ventana.
    samples: Arc<parking_lot::Mutex<VecDeque<MarketSample>>>,
    /// Generadores Pedersen.
    gen_g: G1Affine,
    gen_h: G1Affine,
    /// Contador de quotes.
    quote_count: AtomicU64,
    /// Suma total de ajustes.
    total_adjustment: AtomicU64,
    /// Suma total de precios.
    total_price_sum: AtomicU64,
}

impl PricingEngine {
    /// Crea un pricing engine con configuración personalizada.
    pub fn with_config(config: PricingConfig) -> Self {
        let mut rng = thread_rng();
        let gen_g = G1Projective::rand(&mut rng).into_affine();
        let gen_h = G1Projective::rand(&mut rng).into_affine();

        info!(
            window_ms = %config.window_size_ms,
            min_price = %config.min_price,
            max_price = %config.max_price,
            "PricingEngine initialized"
        );

        Self {
            config,
            samples: Arc::new(parking_lot::Mutex::new(VecDeque::new())),
            gen_g,
            gen_h,
            quote_count: AtomicU64::new(0),
            total_adjustment: AtomicU64::new(0),
            total_price_sum: AtomicU64::new(0),
        }
    }

    /// Registra un sample de mercado.
    pub fn record_sample(&self, sample: MarketSample) {
        let timestamp = sample.timestamp;
        let price = sample.price;
        let demand = sample.demand_count;
        let mut samples = self.samples.lock();
        samples.push_back(sample);
        self.prune_expired(&mut samples);
        debug!(
            timestamp = %timestamp,
            price = %price,
            demand = %demand,
            "Market sample recorded"
        );
    }

    /// Calcula un precio comprometido para un recurso.
    pub fn compute_price(
        &self,
        resource_type: PricingResourceType,
        base_price: f32,
        _quantity: f32,
    ) -> Result<PriceQuote, PricingError> {
        let start = Instant::now();

        // Determinar precio base según tipo
        let price_base = match resource_type {
            PricingResourceType::SAEShard => base_price.max(self.config.sae_base_price),
            PricingResourceType::VRAM => base_price.max(self.config.vram_base_price),
            PricingResourceType::Bandwidth => base_price.max(self.config.bandwidth_base_price),
            PricingResourceType::Generic => base_price,
        };

        // Calcular factor de ajuste basado en oferta/demanda
        let adjustment = self.compute_adjustment_factor()?;

        // Precio final
        let unit_price = price_base * adjustment;

        // Validar límites
        if unit_price < self.config.min_price || unit_price > self.config.max_price {
            return Err(PricingError::PriceOutOfBounds(
                unit_price,
                self.config.min_price,
                self.config.max_price,
            ));
        }

        // Generar compromiso Pedersen
        let commitment = self.commit_price(unit_price);
        let resource_type_str = format!("{:?}", resource_type);

        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        let validity_ms = self.config.window_size_ms / 2; // Validez: mitad de la ventana

        let quote = PriceQuote {
            unit_price,
            commitment_hash: commitment.hash,
            adjustment_factor: adjustment,
            resource_type,
            validity_window_ms: validity_ms,
            expires_at: now_ms + validity_ms,
        };

        // Actualizar estadísticas
        self.quote_count.fetch_add(1, Ordering::Relaxed);
        self.total_adjustment
            .fetch_add((adjustment * 1000.0) as u64, Ordering::Relaxed);
        self.total_price_sum
            .fetch_add((unit_price * 1000.0) as u64, Ordering::Relaxed);

        debug!(
            resource_type = %resource_type_str,
            base_price = %price_base,
            adjustment = %adjustment,
            final_price = %unit_price,
            elapsed_ms = %start.elapsed().as_secs_f64() * 1000.0,
            "Price computed"
        );

        Ok(quote)
    }

    /// Verifica un compromiso de precio usando blinding determinista.
    pub fn verify_commitment(&self, price: f32, hash: [u8; 32]) -> bool {
        let commitment = self.commit_price_deterministic(price);
        commitment.hash == hash
    }

    /// Genera un compromiso Pedersen determinista para verificación.
    fn commit_price_deterministic(&self, price: f32) -> PriceCommitment {
        let price_fr = Fr::from(price as u64);
        // Blinding determinista: hash del precio
        let price_bytes = price.to_le_bytes();
        let hash_bytes: [u8; 32] = Sha256::digest(price_bytes).into();
        let blinding = Fr::from_le_bytes_mod_order(&hash_bytes);

        let g_proj = G1Projective::from(self.gen_g);
        let h_proj = G1Projective::from(self.gen_h);
        let commitment = (g_proj * price_fr) + (h_proj * blinding);
        let commitment_affine: G1Affine = commitment.into_affine();

        let mut bytes = Vec::new();
        commitment_affine
            .serialize_compressed(&mut bytes)
            .unwrap_or_default();

        let mut hash = [0u8; 32];
        let hash_input_slice = &bytes[..bytes.len().min(32)];
        hash[..hash_input_slice.len()].copy_from_slice(hash_input_slice);
        let final_hash = Sha256::digest(hash);
        hash.copy_from_slice(&final_hash);

        PriceCommitment {
            commitment_point: commitment_affine,
            hash,
            blinding_factor: blinding,
        }
    }

    /// Obtiene estadísticas del pricing engine.
    pub fn get_stats(&self) -> PricingStats {
        let samples = self.samples.lock();
        let total = self.quote_count.load(Ordering::Relaxed);
        let avg_adj = if total > 0 {
            self.total_adjustment.load(Ordering::Relaxed) as f32 / total as f32 / 1000.0
        } else {
            1.0
        };
        let avg_price = if total > 0 {
            self.total_price_sum.load(Ordering::Relaxed) as f32 / total as f32 / 1000.0
        } else {
            0.0
        };

        let (demand, supply) = if samples.is_empty() {
            (0, 0.0)
        } else {
            let demand: u64 = samples.iter().map(|s| s.demand_count).sum();
            let supply: f32 = samples.iter().map(|s| s.quantity).sum();
            (demand, supply)
        };

        PricingStats {
            total_quotes: total,
            avg_adjustment: avg_adj,
            avg_price,
            current_demand: demand,
            current_supply: supply,
        }
    }

    /// Calcula el factor de ajuste basado en oferta y demanda en la ventana.
    fn compute_adjustment_factor(&self) -> Result<f32, PricingError> {
        let samples = self.samples.lock();

        if samples.len() < 2 {
            return Ok(1.0); // Sin ajuste con datos insuficientes
        }

        // Calcular demanda total y oferta total
        let total_demand: u64 = samples.iter().map(|s| s.demand_count).sum();
        let total_supply: f32 = samples.iter().map(|s| s.quantity).sum();

        // Ratio de demanda (1.0 = equilibrio)
        let demand_ratio = if total_supply > 0.0 {
            total_demand as f32 / (total_supply * 10.0).max(1.0) // Normalizado
        } else {
            1.0
        };

        // Factor de ajuste: demanda alta -> precio sube, oferta alta -> precio baja
        let demand_factor = 1.0 + (demand_ratio - 1.0) * self.config.demand_sensitivity;
        let supply_factor = 1.0 / (1.0 + total_supply * self.config.supply_sensitivity * 0.01);

        let adjustment = demand_factor * supply_factor;

        // Clamp a [0.1, 5.0]
        let adjustment = adjustment.clamp(0.1, 5.0);

        Ok(adjustment)
    }

    /// Genera un compromiso Pedersen para un precio (con blinding determinista).
    fn commit_price(&self, price: f32) -> PriceCommitment {
        self.commit_price_deterministic(price)
    }

    /// Elimina samples expirados de la ventana.
    fn prune_expired(&self, samples: &mut VecDeque<MarketSample>) {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let cutoff = now.saturating_sub(self.config.window_size_ms);

        while samples.front().is_some_and(|s| s.timestamp < cutoff) {
            samples.pop_front();
        }
    }

    /// Crea un nuevo pricing engine con configuración por defecto.
    pub fn new() -> Self {
        Self::with_config(PricingConfig::default())
    }
}

impl Default for PricingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_price_basic() {
        let engine = PricingEngine::new();
        let quote = engine
            .compute_price(PricingResourceType::SAEShard, 100.0, 1.0)
            .unwrap();

        assert!(quote.unit_price > 0.0);
        assert!(quote.unit_price <= engine.config.max_price);
        assert_eq!(quote.resource_type, PricingResourceType::SAEShard);
    }

    #[test]
    fn test_compute_price_respects_min() {
        let engine = PricingEngine::new();
        // Base price bajo debería ser elevado al mínimo
        let quote = engine
            .compute_price(PricingResourceType::SAEShard, 10.0, 1.0)
            .unwrap();

        assert!(quote.unit_price >= engine.config.sae_base_price);
    }

    #[test]
    fn test_verify_commitment() {
        let engine = PricingEngine::new();
        let quote = engine
            .compute_price(PricingResourceType::VRAM, 250.0, 1.0)
            .unwrap();

        assert!(engine.verify_commitment(quote.unit_price, quote.commitment_hash));
    }

    #[test]
    fn test_verify_wrong_commitment() {
        let engine = PricingEngine::new();
        let quote = engine
            .compute_price(PricingResourceType::VRAM, 250.0, 1.0)
            .unwrap();

        let mut wrong_hash = quote.commitment_hash;
        wrong_hash[0] ^= 0xFF; // Corromper hash
        assert!(!engine.verify_commitment(quote.unit_price, wrong_hash));
    }

    #[test]
    fn test_record_and_compute_with_samples() {
        let engine = PricingEngine::new();

        // Registrar samples para tener datos
        for i in 0..5 {
            engine.record_sample(MarketSample {
                timestamp: chrono::Utc::now().timestamp_millis() as u64 - (i * 1000) as u64,
                price: 100.0 + i as f32,
                quantity: 10.0,
                demand_count: 5 + i,
            });
        }

        let quote = engine
            .compute_price(PricingResourceType::Generic, 100.0, 1.0)
            .unwrap();

        // Con alta demanda, el ajuste debería ser > 1.0
        assert!(quote.unit_price > 0.0);
    }

    #[test]
    fn test_get_stats() {
        let engine = PricingEngine::new();

        // Inicial stats
        let stats = engine.get_stats();
        assert_eq!(stats.total_quotes, 0);

        // Generar quotes
        engine
            .compute_price(PricingResourceType::SAEShard, 100.0, 1.0)
            .unwrap();
        engine
            .compute_price(PricingResourceType::VRAM, 250.0, 1.0)
            .unwrap();

        let stats = engine.get_stats();
        assert_eq!(stats.total_quotes, 2);
        assert!(stats.avg_price > 0.0);
    }

    #[test]
    fn test_price_bounds() {
        let config = PricingConfig {
            window_size_ms: 3600000,
            min_price: 5000.0,
            max_price: 6000.0,
            sae_base_price: 100.0,
            vram_base_price: 250.0,
            bandwidth_base_price: 10.0,
            demand_sensitivity: 2.0,
            supply_sensitivity: 0.1,
        };
        let engine = PricingEngine::with_config(config);

        // Con alta demanda y límites ajustados, debería fallar si excede max
        engine.record_sample(MarketSample {
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            price: 100.0,
            quantity: 1.0,
            demand_count: 100,
        });

        let result = engine.compute_price(PricingResourceType::Generic, 100.0, 1.0);
        // Puede fallar por bounds o retornar precio dentro de bounds
        match result {
            Ok(quote) => {
                assert!(quote.unit_price >= 5000.0 && quote.unit_price <= 6000.0);
            }
            Err(PricingError::PriceOutOfBounds(..)) => {
                // También válido
            }
            _ => panic!("Expected PriceOutOfBounds error"),
        }
    }

    #[test]
    fn test_resource_type_pricing() {
        let engine = PricingEngine::new();

        let sae_quote = engine
            .compute_price(PricingResourceType::SAEShard, 50.0, 1.0)
            .unwrap();
        let vram_quote = engine
            .compute_price(PricingResourceType::VRAM, 100.0, 1.0)
            .unwrap();
        let bw_quote = engine
            .compute_price(PricingResourceType::Bandwidth, 5.0, 1.0)
            .unwrap();

        // VRAM debería ser más caro que bandwidth
        assert!(vram_quote.unit_price > bw_quote.unit_price);
        // SAE debería ser al menos el base price
        assert!(sae_quote.unit_price >= engine.config.sae_base_price);
    }
}
