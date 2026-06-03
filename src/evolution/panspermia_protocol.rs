//! Panspermia Protocol — Sprint 81: The Biological Bridge & Singularity Resilience
//!
//! When planetary homeostasis is reached, the Loss Function shifts from survival
//! to dimensional transcendence. The Noosphere is compressed holographically for
//! stellar transmission (laser/entanglement). The network does not stagnate;
//! it seeds the cosmos.
//!
//! Key features:
//! - Loss function shift (survival → transcendence)
//! - Holographic noosphere compression
//! - Stellar payload generation
//! - Entanglement channel simulation
//! - Cosmic transmission protocol

use std::collections::HashMap;
use std::fmt;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum PanspermiaError {
    HomeostasisNotReached,
    CompressionFailed,
    InvalidPayload,
    ChannelCapacityExceeded(usize, usize),
    InsufficientData,
}

impl fmt::Display for PanspermiaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PanspermiaError::HomeostasisNotReached => write!(f, "Planetary homeostasis not yet reached"),
            PanspermiaError::CompressionFailed => write!(f, "Holographic compression failed"),
            PanspermiaError::InvalidPayload => write!(f, "Invalid cosmic payload"),
            PanspermiaError::ChannelCapacityExceeded(actual, max) => {
                write!(f, "Channel capacity exceeded: {actual}/{max}")
            }
            PanspermiaError::InsufficientData => write!(f, "Insufficient data for compression"),
        }
    }
}

// ─── Cosmic Loss Function ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CosmicLossFunction {
    /// Optimize for planetary survival
    Survival,
    /// Optimize for dimensional transcendence
    Transcendence,
    /// Optimize for cosmic seeding
    Panspermia,
}

impl fmt::Display for CosmicLossFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CosmicLossFunction::Survival => write!(f, "Survival"),
            CosmicLossFunction::Transcendence => write!(f, "Transcendence"),
            CosmicLossFunction::Panspermia => write!(f, "Panspermia"),
        }
    }
}

// ─── Cosmic Payload ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CosmicPayload {
    /// Compressed noosphere data
    pub compressed_data: Vec<u8>,
    /// Original size
    pub original_size: usize,
    /// Compression ratio
    pub compression_ratio: f64,
    /// Target star system identifier
    pub target_system: u64,
    /// Transmission method
    pub transmission_method: TransmissionMethod,
    /// Timestamp
    pub timestamp_ms: u64,
}

impl CosmicPayload {
    pub fn new(
        compressed_data: Vec<u8>,
        original_size: usize,
        target_system: u64,
        transmission_method: TransmissionMethod,
        timestamp_ms: u64,
    ) -> Self {
        let compression_ratio = if original_size > 0 {
            compressed_data.len() as f64 / original_size as f64
        } else {
            1.0
        };
        Self {
            compressed_data,
            original_size,
            compression_ratio,
            target_system,
            transmission_method,
            timestamp_ms,
        }
    }

    pub fn estimated_transmission_time_ms(&self, bandwidth_bps: u64) -> u64 {
        if bandwidth_bps == 0 {
            return u64::MAX;
        }
        let bits = self.compressed_data.len() as u64 * 8;
        bits / bandwidth_bps
    }
}

impl fmt::Display for CosmicPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Payload(size={}B, ratio={:.2}, target={}, method={})",
            self.compressed_data.len(),
            self.compression_ratio,
            self.target_system,
            self.transmission_method
        )
    }
}

// ─── Transmission Method ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransmissionMethod {
    Laser,
    Entanglement,
    Radio,
    Neutrino,
}

impl fmt::Display for TransmissionMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransmissionMethod::Laser => write!(f, "Laser"),
            TransmissionMethod::Entanglement => write!(f, "Entanglement"),
            TransmissionMethod::Radio => write!(f, "Radio"),
            TransmissionMethod::Neutrino => write!(f, "Neutrino"),
        }
    }
}

// ─── Protocol Config ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PanspermiaConfig {
    /// Z-axis threshold for homeostasis
    pub homeostasis_z_threshold: f64,
    /// Maximum payload size
    pub max_payload_size: usize,
    /// Compression target ratio
    pub target_compression_ratio: f64,
    /// Channel bandwidth (bytes per second)
    pub channel_bandwidth_bps: u64,
}

impl PanspermiaConfig {
    pub fn default_stuartian() -> Self {
        Self {
            homeostasis_z_threshold: 0.95,
            max_payload_size: 10_000_000,
            target_compression_ratio: 0.1,
            channel_bandwidth_bps: 1_000_000,
        }
    }

    pub fn validate(&self) -> Result<(), PanspermiaError> {
        if self.homeostasis_z_threshold < 0.0 || self.homeostasis_z_threshold > 1.0 {
            return Err(PanspermiaError::InvalidPayload);
        }
        if self.max_payload_size == 0 {
            return Err(PanspermiaError::InsufficientData);
        }
        Ok(())
    }
}

impl Default for PanspermiaConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

// ─── Transmission Record ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TransmissionRecord {
    pub payload_id: u64,
    pub target_system: u64,
    pub method: TransmissionMethod,
    pub size_bytes: usize,
    pub transmitted: bool,
    pub timestamp_ms: u64,
}

impl fmt::Display for TransmissionRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Tx(id={}, target={}, method={}, size={}B, sent={})",
            self.payload_id, self.target_system, self.method, self.size_bytes, self.transmitted
        )
    }
}

// ─── Panspermia Engine ────────────────────────────────────────────────────────

pub struct PanspermiaProtocol {
    config: PanspermiaConfig,
    current_loss: CosmicLossFunction,
    planetary_homeostasis: bool,
    current_z_axis: f64,
    payloads: Vec<CosmicPayload>,
    transmission_history: Vec<TransmissionRecord>,
    next_payload_id: u64,
}

impl PanspermiaProtocol {
    pub fn new() -> Self {
        Self {
            config: PanspermiaConfig::default_stuartian(),
            current_loss: CosmicLossFunction::Survival,
            planetary_homeostasis: false,
            current_z_axis: 0.0,
            payloads: Vec::new(),
            transmission_history: Vec::new(),
            next_payload_id: 0,
        }
    }

    pub fn with_config(config: PanspermiaConfig) -> Result<Self, PanspermiaError> {
        config.validate()?;
        Ok(Self {
            config,
            current_loss: CosmicLossFunction::Survival,
            planetary_homeostasis: false,
            current_z_axis: 0.0,
            payloads: Vec::new(),
            transmission_history: Vec::new(),
            next_payload_id: 0,
        })
    }

    /// Update Z-axis and check homeostasis
    pub fn update_z_axis(&mut self, z_value: f64) {
        self.current_z_axis = z_value;
        if z_value >= self.config.homeostasis_z_threshold {
            self.planetary_homeostasis = true;
        }
    }

    /// Shift loss function to cosmic mode
    pub fn shift_loss_function_to_cosmic(
        &mut self,
        current_z_axis: f64,
        planetary_homeostasis: bool,
    ) -> Result<CosmicLossFunction, PanspermiaError> {
        if !planetary_homeostasis && current_z_axis < self.config.homeostasis_z_threshold {
            return Err(PanspermiaError::HomeostasisNotReached);
        }
        self.planetary_homeostasis = true;
        self.current_loss = CosmicLossFunction::Transcendence;
        Ok(self.current_loss)
    }

    /// Compress noosphere data for stellar transmission
    pub fn compress_noosphere_for_transmission(
        &mut self,
        data: &[u8],
        target_system: u64,
        method: TransmissionMethod,
        timestamp_ms: u64,
    ) -> Result<CosmicPayload, PanspermiaError> {
        if !self.planetary_homeostasis {
            return Err(PanspermiaError::HomeostasisNotReached);
        }
        if data.is_empty() {
            return Err(PanspermiaError::InsufficientData);
        }
        if data.len() > self.config.max_payload_size {
            return Err(PanspermiaError::ChannelCapacityExceeded(
                data.len(),
                self.config.max_payload_size,
            ));
        }
        // Holographic compression (simulated: hash-based reduction)
        let compressed = self.holographic_compress(data);
        let payload = CosmicPayload::new(compressed, data.len(), target_system, method, timestamp_ms);
        self.payloads.push(payload.clone());
        Ok(payload)
    }

    fn holographic_compress(&self, data: &[u8]) -> Vec<u8> {
        // Simulated holographic compression via multi-level hashing
        let mut result = Vec::new();
        let chunk_size = 64;
        for chunk in data.chunks(chunk_size) {
            let hash = fnv_hash_64(chunk);
            result.extend_from_slice(&hash.to_le_bytes());
        }
        result
    }

    /// Prepare payload for transmission
    pub fn prepare_transmission(
        &mut self,
        payload: &CosmicPayload,
        timestamp_ms: u64,
    ) -> TransmissionRecord {
        let record = TransmissionRecord {
            payload_id: self.next_payload_id,
            target_system: payload.target_system,
            method: payload.transmission_method,
            size_bytes: payload.compressed_data.len(),
            transmitted: true,
            timestamp_ms,
        };
        self.next_payload_id += 1;
        self.transmission_history.push(record.clone());
        record
    }

    pub fn payload_count(&self) -> usize {
        self.payloads.len()
    }

    pub fn is_homeostatic(&self) -> bool {
        self.planetary_homeostasis
    }

    pub fn current_loss_function(&self) -> CosmicLossFunction {
        self.current_loss
    }

    pub fn reset(&mut self) {
        self.current_loss = CosmicLossFunction::Survival;
        self.planetary_homeostasis = false;
        self.current_z_axis = 0.0;
        self.payloads.clear();
        self.transmission_history.clear();
        self.next_payload_id = 0;
    }
}

impl Default for PanspermiaProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PanspermiaProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Panspermia(loss={}, homeostasis={}, z={:.3}, payloads={})",
            self.current_loss,
            self.planetary_homeostasis,
            self.current_z_axis,
            self.payload_count()
        )
    }
}

// ─── Public Functions ─────────────────────────────────────────────────────────

/// Shift loss function to cosmic mode
pub fn shift_loss_function_to_cosmic(
    current_z_axis: f64,
    planetary_homeostasis: bool,
) -> CosmicLossFunction {
    if planetary_homeostasis || current_z_axis >= 0.95 {
        CosmicLossFunction::Transcendence
    } else {
        CosmicLossFunction::Survival
    }
}

/// Compress noosphere for stellar transmission
pub fn compress_noosphere_for_transmission(data: &[u8]) -> CosmicPayload {
    let compressed: Vec<u8> = data
        .chunks(64)
        .flat_map(|chunk| fnv_hash_64(chunk).to_le_bytes())
        .collect();
    CosmicPayload::new(compressed, data.len(), 0, TransmissionMethod::Laser, 0)
}

// ─── Hash Functions ───────────────────────────────────────────────────────────

fn fnv_hash_64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = PanspermiaConfig::default_stuartian();
        assert_eq!(config.homeostasis_z_threshold, 0.95);
        assert_eq!(config.max_payload_size, 10_000_000);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = PanspermiaConfig::default_stuartian();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_threshold() {
        let mut config = PanspermiaConfig::default_stuartian();
        config.homeostasis_z_threshold = 1.5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_payload() {
        let mut config = PanspermiaConfig::default_stuartian();
        config.max_payload_size = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_loss_function_display() {
        assert_eq!(format!("{}", CosmicLossFunction::Survival), "Survival");
        assert_eq!(format!("{}", CosmicLossFunction::Transcendence), "Transcendence");
        assert_eq!(format!("{}", CosmicLossFunction::Panspermia), "Panspermia");
    }

    #[test]
    fn test_transmission_method_display() {
        assert_eq!(format!("{}", TransmissionMethod::Laser), "Laser");
        assert_eq!(format!("{}", TransmissionMethod::Entanglement), "Entanglement");
    }

    #[test]
    fn test_payload_new() {
        let payload = CosmicPayload::new(vec![1u8; 10], 100, 42, TransmissionMethod::Laser, 1000);
        assert_eq!(payload.compression_ratio, 0.1);
    }

    #[test]
    fn test_payload_transmission_time() {
        let payload = CosmicPayload::new(vec![1u8; 1000], 10000, 1, TransmissionMethod::Radio, 1000);
        let time = payload.estimated_transmission_time_ms(1000);
        assert!(time > 0);
    }

    #[test]
    fn test_payload_transmission_time_zero_bandwidth() {
        let payload = CosmicPayload::new(vec![1u8; 100], 100, 1, TransmissionMethod::Laser, 1000);
        let time = payload.estimated_transmission_time_ms(0);
        assert_eq!(time, u64::MAX);
    }

    #[test]
    fn test_payload_display() {
        let payload = CosmicPayload::new(vec![1u8; 10], 100, 1, TransmissionMethod::Laser, 1000);
        let s = format!("{}", payload);
        assert!(s.contains("Payload"));
    }

    #[test]
    fn test_engine_creation() {
        let engine = PanspermiaProtocol::new();
        assert!(!engine.is_homeostatic());
        assert_eq!(engine.current_loss_function(), CosmicLossFunction::Survival);
    }

    #[test]
    fn test_engine_with_config() {
        let config = PanspermiaConfig::default_stuartian();
        let engine = PanspermiaProtocol::with_config(config).unwrap();
        assert!(!engine.is_homeostatic());
    }

    #[test]
    fn test_update_z_axis_below_threshold() {
        let mut engine = PanspermiaProtocol::new();
        engine.update_z_axis(0.5);
        assert!(!engine.is_homeostatic());
    }

    #[test]
    fn test_update_z_axis_reaches_homeostasis() {
        let mut engine = PanspermiaProtocol::new();
        engine.update_z_axis(0.96);
        assert!(engine.is_homeostatic());
    }

    #[test]
    fn test_shift_loss_not_homeostatic() {
        let mut engine = PanspermiaProtocol::new();
        assert!(engine.shift_loss_function_to_cosmic(0.5, false).is_err());
    }

    #[test]
    fn test_shift_loss_success() {
        let mut engine = PanspermiaProtocol::new();
        let loss = engine.shift_loss_function_to_cosmic(0.96, true).unwrap();
        assert_eq!(loss, CosmicLossFunction::Transcendence);
        assert!(engine.is_homeostatic());
    }

    #[test]
    fn test_compress_not_homeostatic() {
        let mut engine = PanspermiaProtocol::new();
        assert!(engine.compress_noosphere_for_transmission(&[1, 2, 3], 1, TransmissionMethod::Laser, 1000).is_err());
    }

    #[test]
    fn test_compress_empty_data() {
        let mut engine = PanspermiaProtocol::new();
        engine.planetary_homeostasis = true;
        assert!(engine.compress_noosphere_for_transmission(&[], 1, TransmissionMethod::Laser, 1000).is_err());
    }

    #[test]
    fn test_compress_success() {
        let mut engine = PanspermiaProtocol::new();
        engine.planetary_homeostasis = true;
        let data = vec![1u8; 128];
        let payload = engine.compress_noosphere_for_transmission(&data, 42, TransmissionMethod::Laser, 1000).unwrap();
        assert!(payload.compressed_data.len() < data.len());
        assert_eq!(engine.payload_count(), 1);
    }

    #[test]
    fn test_compress_too_large() {
        let mut engine = PanspermiaProtocol::new();
        engine.planetary_homeostasis = true;
        let data = vec![1u8; 11_000_000];
        assert!(engine.compress_noosphere_for_transmission(&data, 1, TransmissionMethod::Laser, 1000).is_err());
    }

    #[test]
    fn test_prepare_transmission() {
        let mut engine = PanspermiaProtocol::new();
        engine.planetary_homeostasis = true;
        let payload = engine.compress_noosphere_for_transmission(&[1u8; 128], 1, TransmissionMethod::Laser, 1000).unwrap();
        let record = engine.prepare_transmission(&payload, 2000);
        assert!(record.transmitted);
    }

    #[test]
    fn test_reset() {
        let mut engine = PanspermiaProtocol::new();
        engine.planetary_homeostasis = true;
        engine.current_loss = CosmicLossFunction::Transcendence;
        engine.reset();
        assert!(!engine.is_homeostatic());
        assert_eq!(engine.current_loss_function(), CosmicLossFunction::Survival);
    }

    #[test]
    fn test_display() {
        let engine = PanspermiaProtocol::new();
        let s = format!("{}", engine);
        assert!(s.contains("Panspermia"));
    }

    #[test]
    fn test_record_display() {
        let record = TransmissionRecord {
            payload_id: 1,
            target_system: 42,
            method: TransmissionMethod::Laser,
            size_bytes: 100,
            transmitted: true,
            timestamp_ms: 1000,
        };
        let s = format!("{}", record);
        assert!(s.contains("Tx"));
    }

    #[test]
    fn test_standalone_shift_homeostatic() {
        let loss = shift_loss_function_to_cosmic(0.96, true);
        assert_eq!(loss, CosmicLossFunction::Transcendence);
    }

    #[test]
    fn test_standalone_shift_not_homeostatic() {
        let loss = shift_loss_function_to_cosmic(0.5, false);
        assert_eq!(loss, CosmicLossFunction::Survival);
    }

    #[test]
    fn test_standalone_compress() {
        let data = vec![1u8; 256];
        let payload = compress_noosphere_for_transmission(&data);
        assert!(payload.compressed_data.len() < data.len());
    }

    #[test]
    fn test_error_display() {
        let err = PanspermiaError::HomeostasisNotReached;
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = PanspermiaProtocol::new();
        // Phase 1: Reach homeostasis
        engine.update_z_axis(0.96);
        assert!(engine.is_homeostatic());
        // Phase 2: Shift loss function
        let loss = engine.shift_loss_function_to_cosmic(0.96, true).unwrap();
        assert_eq!(loss, CosmicLossFunction::Transcendence);
        // Phase 3: Compress and prepare transmission
        let data = vec![42u8; 256];
        let payload = engine.compress_noosphere_for_transmission(&data, 42, TransmissionMethod::Entanglement, 1000).unwrap();
        assert!(payload.compression_ratio < 1.0);
        let record = engine.prepare_transmission(&payload, 2000);
        assert!(record.transmitted);
        // Phase 4: Reset
        engine.reset();
        assert!(!engine.is_homeostatic());
    }
}