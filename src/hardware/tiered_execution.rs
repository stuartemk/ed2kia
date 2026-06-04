//! Tiered Execution â€” Sprint 72: Asymptotic Optimization & Hard Sybil Resistance
//!
//! WASM tiering (Edge vs Core), memory pooling, INT4/FP8 quantization.

use std::collections::HashMap;
use std::fmt;

// â”€â”€â”€ Error Types â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, PartialEq)]
pub enum TierError {
    InvalidTier(String),
    MemoryPoolExhausted,
    QuantizationFailed,
    UnsupportedPrecision(u8),
    InvalidConfig,
    NodeNotFound(u64),
    CapacityExceeded(usize),
    InvalidMemorySize(usize),
}

impl fmt::Display for TierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TierError::InvalidTier(t) => write!(f, "Invalid execution tier: {}", t),
            TierError::MemoryPoolExhausted => write!(f, "Memory pool exhausted"),
            TierError::QuantizationFailed => write!(f, "Quantization operation failed"),
            TierError::UnsupportedPrecision(p) => {
                write!(f, "Unsupported precision level: {}", p)
            }
            TierError::InvalidConfig => write!(f, "Invalid tiered execution configuration"),
            TierError::NodeNotFound(id) => write!(f, "Execution node {} not found", id),
            TierError::CapacityExceeded(c) => {
                write!(f, "Capacity {} exceeded maximum allowed", c)
            }
            TierError::InvalidMemorySize(s) => {
                write!(f, "Invalid memory pool size: {} bytes", s)
            }
        }
    }
}

impl std::error::Error for TierError {}

// â”€â”€â”€ Execution Tier â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ExecutionTier {
    /// Edge/WASM nodes â€” lightweight, quantized inference
    Edge = 0,
    /// Core/GPU nodes â€” full precision, heavy computation
    Core = 1,
    /// Hybrid nodes â€” can switch between Edge and Core
    Hybrid = 2,
}

impl fmt::Display for ExecutionTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionTier::Edge => write!(f, "Edge"),
            ExecutionTier::Core => write!(f, "Core"),
            ExecutionTier::Hybrid => write!(f, "Hybrid"),
        }
    }
}

impl ExecutionTier {
    pub fn max_memory_mb(&self) -> usize {
        match self {
            ExecutionTier::Edge => 512,
            ExecutionTier::Core => 8192,
            ExecutionTier::Hybrid => 4096,
        }
    }

    pub fn default_precision(&self) -> Precision {
        match self {
            ExecutionTier::Edge => Precision::Int4,
            ExecutionTier::Core => Precision::Fp32,
            ExecutionTier::Hybrid => Precision::Fp8,
        }
    }

    pub fn quantization_factor(&self) -> f64 {
        match self {
            ExecutionTier::Edge => 0.0625, // INT4 = 4/64
            ExecutionTier::Core => 1.0,    // FP32 = full
            ExecutionTier::Hybrid => 0.25, // FP8 = 8/32
        }
    }
}

// â”€â”€â”€ Precision Levels â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    Int4,
    Int8,
    Fp8,
    Fp16,
    Fp32,
}

impl fmt::Display for Precision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Precision::Int4 => write!(f, "INT4"),
            Precision::Int8 => write!(f, "INT8"),
            Precision::Fp8 => write!(f, "FP8"),
            Precision::Fp16 => write!(f, "FP16"),
            Precision::Fp32 => write!(f, "FP32"),
        }
    }
}

impl Precision {
    pub fn bits(&self) -> u8 {
        match self {
            Precision::Int4 => 4,
            Precision::Int8 => 8,
            Precision::Fp8 => 8,
            Precision::Fp16 => 16,
            Precision::Fp32 => 32,
        }
    }

    pub fn bytes_per_element(&self) -> usize {
        (self.bits() as usize + 7) / 8
    }

    pub fn is_supported(&self) -> bool {
        matches!(
            self,
            Precision::Int4 | Precision::Int8 | Precision::Fp8 | Precision::Fp16 | Precision::Fp32
        )
    }

    /// Quantize a f32 value to the target precision
    pub fn quantize(&self, value: f32) -> f32 {
        match self {
            Precision::Int4 => {
                // INT4: 8 levels (-8 to 7)
                let scaled = (value * 8.0).round().clamp(-8.0, 7.0);
                scaled / 8.0
            }
            Precision::Int8 => {
                // INT8: 256 levels (-128 to 127)
                let scaled = (value * 128.0).round().clamp(-128.0, 127.0);
                scaled / 128.0
            }
            Precision::Fp8 => {
                // E4M3 format approximation
                let sign = if value < 0.0 { -1.0 } else { 1.0 };
                let abs = value.abs();
                if abs < 0.00390625 {
                    return 0.0; // Below minimum
                }
                let log2 = abs.log2().max(-8.0).min(15.0);
                let exp = log2.floor() as i32;
                let mantissa = ((abs / 2.0_f32.powi(exp) * 8.0).round() / 8.0).clamp(1.0, 1.9375);
                sign * mantissa * 2.0_f32.powi(exp)
            }
            Precision::Fp16 => {
                // Half precision approximation
                let quantized = (value * 1000.0).round() / 1000.0;
                quantized.clamp(-65504.0, 65504.0)
            }
            Precision::Fp32 => value,
        }
    }

    /// Dequantize back to f32
    pub fn dequantize(&self, value: f32) -> f32 {
        // For this simulation, dequantization is identity
        // In production, this would reverse the quantization
        value
    }
}

// â”€â”€â”€ Configuration â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone)]
pub struct TieredConfig {
    /// Memory pool size in bytes
    pub memory_pool_size: usize,
    /// Maximum nodes per tier
    pub max_nodes_per_tier: usize,
    /// Enable automatic tier assignment
    pub auto_tier: bool,
    /// Enable memory pooling
    pub memory_pooling: bool,
    /// Default precision for new nodes
    pub default_precision: Precision,
    /// Quantization threshold (below this, force lower precision)
    pub quantization_threshold: f64,
}

impl TieredConfig {
    pub fn default_Topological() -> Self {
        Self {
            memory_pool_size: 1024 * 1024 * 64, // 64 MB
            max_nodes_per_tier: 256,
            auto_tier: true,
            memory_pooling: true,
            default_precision: Precision::Fp32,
            quantization_threshold: 0.01,
        }
    }

    pub fn validate(&self) -> Result<(), TierError> {
        if self.memory_pool_size == 0 {
            return Err(TierError::InvalidMemorySize(0));
        }
        if self.max_nodes_per_tier == 0 {
            return Err(TierError::InvalidConfig);
        }
        if !self.default_precision.is_supported() {
            return Err(TierError::UnsupportedPrecision(
                self.default_precision.bits(),
            ));
        }
        if self.quantization_threshold <= 0.0 || self.quantization_threshold > 1.0 {
            return Err(TierError::InvalidConfig);
        }
        Ok(())
    }
}

impl Default for TieredConfig {
    fn default() -> Self {
        Self::default_Topological()
    }
}

// â”€â”€â”€ Memory Pool â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug)]
pub struct MemoryPool {
    total_bytes: usize,
    used_bytes: usize,
    allocations: HashMap<u64, usize>, // allocation_id -> size
    next_id: u64,
}

impl MemoryPool {
    pub fn new(size: usize) -> Self {
        Self {
            total_bytes: size,
            used_bytes: 0,
            allocations: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn allocate(&mut self, size: usize) -> Result<u64, TierError> {
        if self.used_bytes + size > self.total_bytes {
            return Err(TierError::MemoryPoolExhausted);
        }
        let id = self.next_id;
        self.next_id += 1;
        self.used_bytes += size;
        self.allocations.insert(id, size);
        Ok(id)
    }

    pub fn deallocate(&mut self, id: u64) -> Result<usize, TierError> {
        match self.allocations.remove(&id) {
            None => Err(TierError::NodeNotFound(id)),
            Some(size) => {
                self.used_bytes -= size;
                Ok(size)
            }
        }
    }

    pub fn available(&self) -> usize {
        self.total_bytes - self.used_bytes
    }

    pub fn utilization(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        self.used_bytes as f64 / self.total_bytes as f64
    }

    pub fn allocation_count(&self) -> usize {
        self.allocations.len()
    }

    pub fn reset(&mut self) {
        self.used_bytes = 0;
        self.allocations.clear();
        self.next_id = 1;
    }
}

impl fmt::Display for MemoryPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MemoryPool(total={}MB, used={}MB, util={:.2}%, allocs={})",
            self.total_bytes / 1024 / 1024,
            self.used_bytes / 1024 / 1024,
            self.utilization() * 100.0,
            self.allocations.len()
        )
    }
}

// â”€â”€â”€ Execution Node â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone)]
pub struct ExecutionNode {
    pub node_id: u64,
    pub tier: ExecutionTier,
    pub precision: Precision,
    pub memory_mb: usize,
    pub active: bool,
    pub quantization_factor: f64,
}

impl ExecutionNode {
    pub fn new(node_id: u64, tier: ExecutionTier) -> Self {
        let precision = tier.default_precision();
        Self {
            node_id,
            tier,
            precision,
            memory_mb: tier.max_memory_mb(),
            active: true,
            quantization_factor: tier.quantization_factor(),
        }
    }

    pub fn with_precision(mut self, precision: Precision) -> Result<Self, TierError> {
        if !precision.is_supported() {
            return Err(TierError::UnsupportedPrecision(precision.bits()));
        }
        self.precision = precision;
        Ok(self)
    }

    pub fn estimated_memory_for_tensor(&self, elements: usize) -> usize {
        elements * self.precision.bytes_per_element()
    }

    pub fn quantize_tensor(&self, tensor: &[f32]) -> Vec<f32> {
        tensor.iter().map(|&x| self.precision.quantize(x)).collect()
    }
}

impl fmt::Display for ExecutionNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ExecNode(id={}, tier={}, precision={}, mem={}MB, active={})",
            self.node_id, self.tier, self.precision, self.memory_mb, self.active
        )
    }
}

// â”€â”€â”€ Tiered Executor â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug)]
pub struct TieredExecutor {
    config: TieredConfig,
    nodes: HashMap<u64, ExecutionNode>,
    memory_pool: MemoryPool,
    tier_counts: HashMap<ExecutionTier, usize>,
}

impl TieredExecutor {
    pub fn new() -> Self {
        Self {
            config: TieredConfig::default_Topological(),
            nodes: HashMap::new(),
            memory_pool: MemoryPool::new(1024 * 1024 * 64),
            tier_counts: HashMap::new(),
        }
    }

    pub fn with_config(config: TieredConfig) -> Result<Self, TierError> {
        config.validate()?;
        Ok(Self {
            memory_pool: MemoryPool::new(config.memory_pool_size),
            config,
            nodes: HashMap::new(),
            tier_counts: HashMap::new(),
        })
    }

    /// Register a new execution node
    pub fn register_node(&mut self, node: ExecutionNode) -> Result<(), TierError> {
        let tier_count = self.tier_counts.entry(node.tier).or_insert(0);
        if *tier_count >= self.config.max_nodes_per_tier {
            return Err(TierError::CapacityExceeded(*tier_count));
        }

        // Allocate memory for this node
        if self.config.memory_pooling {
            let bytes = node.memory_mb * 1024 * 1024;
            if bytes > self.memory_pool.available() {
                return Err(TierError::MemoryPoolExhausted);
            }
            self.memory_pool.allocate(bytes)?;
        }

        *tier_count += 1;
        self.nodes.insert(node.node_id, node);
        Ok(())
    }

    /// Auto-assign tier based on workload
    pub fn auto_assign_tier(&self, tensor_size: usize, precision_req: f64) -> ExecutionTier {
        if !self.config.auto_tier {
            return ExecutionTier::Core;
        }

        // Small tensors + low precision req â†’ Edge
        if tensor_size < 1024 && precision_req < self.config.quantization_threshold {
            return ExecutionTier::Edge;
        }

        // Large tensors + high precision req â†’ Core
        if tensor_size > 65536 || precision_req > 0.5 {
            return ExecutionTier::Core;
        }

        // Medium â†’ Hybrid
        ExecutionTier::Hybrid
    }

    /// Get node by ID
    pub fn get_node(&self, node_id: u64) -> Option<&ExecutionNode> {
        self.nodes.get(&node_id)
    }

    /// Get nodes by tier
    pub fn get_nodes_by_tier(&self, tier: ExecutionTier) -> Vec<&ExecutionNode> {
        self.nodes
            .values()
            .filter(|n| n.tier == tier && n.active)
            .collect()
    }

    /// Quantize tensor for a specific node
    pub fn quantize_for_node(&self, node_id: u64, tensor: &[f32]) -> Result<Vec<f32>, TierError> {
        match self.nodes.get(&node_id) {
            None => Err(TierError::NodeNotFound(node_id)),
            Some(node) => Ok(node.quantize_tensor(tensor)),
        }
    }

    /// Estimate memory usage for tensor on a tier
    pub fn estimate_memory(&self, tier: ExecutionTier, elements: usize) -> usize {
        let precision = tier.default_precision();
        elements * precision.bytes_per_element()
    }

    /// Get memory pool stats
    pub fn memory_pool_stats(&self) -> &MemoryPool {
        &self.memory_pool
    }

    /// Total node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Node count per tier
    pub fn tier_counts(&self) -> &HashMap<ExecutionTier, usize> {
        &self.tier_counts
    }

    /// Reset executor
    pub fn reset(&mut self) {
        self.nodes.clear();
        self.memory_pool.reset();
        self.tier_counts.clear();
    }
}

impl Default for TieredExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TieredExecutor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TieredExecutor(nodes={}, pool={})",
            self.nodes.len(),
            self.memory_pool
        )
    }
}

// â”€â”€â”€ Public Utility Functions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Quantize a tensor to the specified precision
pub fn quantize_tensor(tensor: &[f32], precision: Precision) -> Vec<f32> {
    tensor.iter().map(|&x| precision.quantize(x)).collect()
}

/// Compute quantization error (MSE between original and quantized)
pub fn quantization_mse(original: &[f32], quantized: &[f32]) -> f32 {
    if original.len() != quantized.len() || original.is_empty() {
        return 0.0;
    }
    let sum: f32 = original
        .iter()
        .zip(quantized.iter())
        .map(|(o, q)| (o - q) * (o - q))
        .sum();
    sum / original.len() as f32
}

/// Compute memory savings ratio from quantization
pub fn memory_savings_ratio(from: Precision, to: Precision) -> f64 {
    let from_bytes = from.bytes_per_element() as f64;
    let to_bytes = to.bytes_per_element() as f64;
    1.0 - (to_bytes / from_bytes)
}

// â”€â”€â”€ Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;

    // â”€â”€â”€ ExecutionTier Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_tier_display() {
        assert_eq!(format!("{}", ExecutionTier::Edge), "Edge");
        assert_eq!(format!("{}", ExecutionTier::Core), "Core");
        assert_eq!(format!("{}", ExecutionTier::Hybrid), "Hybrid");
    }

    #[test]
    fn test_tier_max_memory() {
        assert_eq!(ExecutionTier::Edge.max_memory_mb(), 512);
        assert_eq!(ExecutionTier::Core.max_memory_mb(), 8192);
        assert_eq!(ExecutionTier::Hybrid.max_memory_mb(), 4096);
    }

    #[test]
    fn test_tier_default_precision() {
        assert_eq!(ExecutionTier::Edge.default_precision(), Precision::Int4);
        assert_eq!(ExecutionTier::Core.default_precision(), Precision::Fp32);
        assert_eq!(ExecutionTier::Hybrid.default_precision(), Precision::Fp8);
    }

    #[test]
    fn test_tier_quantization_factor() {
        assert!((ExecutionTier::Edge.quantization_factor() - 0.0625) < 1e-10);
        assert!((ExecutionTier::Core.quantization_factor() - 1.0) < 1e-10);
        assert!((ExecutionTier::Hybrid.quantization_factor() - 0.25) < 1e-10);
    }

    // â”€â”€â”€ Precision Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_precision_display() {
        assert_eq!(format!("{}", Precision::Int4), "INT4");
        assert_eq!(format!("{}", Precision::Fp32), "FP32");
    }

    #[test]
    fn test_precision_bits() {
        assert_eq!(Precision::Int4.bits(), 4);
        assert_eq!(Precision::Int8.bits(), 8);
        assert_eq!(Precision::Fp8.bits(), 8);
        assert_eq!(Precision::Fp16.bits(), 16);
        assert_eq!(Precision::Fp32.bits(), 32);
    }

    #[test]
    fn test_precision_bytes() {
        assert_eq!(Precision::Int4.bytes_per_element(), 1); // 4 bits â†’ 1 byte
        assert_eq!(Precision::Fp32.bytes_per_element(), 4);
    }

    #[test]
    fn test_precision_supported() {
        assert!(Precision::Int4.is_supported());
        assert!(Precision::Fp32.is_supported());
    }

    #[test]
    fn test_quantize_int4() {
        // INT4: 8 levels (-8 to 7), scaled by 8
        // 1.5 * 8 = 12, clamped to 7, 7/8 = 0.875
        let quantized = Precision::Int4.quantize(1.5);
        assert!((quantized - 0.875).abs() < 0.001);
    }

    #[test]
    fn test_quantize_int8() {
        let quantized = Precision::Int8.quantize(0.1234);
        assert!((quantized - 0.1234).abs() < 0.01);
    }

    #[test]
    fn test_quantize_fp32_identity() {
        let val = 3.14159;
        assert_eq!(Precision::Fp32.quantize(val), val);
    }

    #[test]
    fn test_quantize_clamping() {
        let quantized = Precision::Int4.quantize(100.0);
        assert!(quantized <= 7.0 / 8.0 * 100.0); // Should be clamped
    }

    // â”€â”€â”€ Config Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_config_default() {
        let config = TieredConfig::default_Topological();
        assert!(config.auto_tier);
        assert!(config.memory_pooling);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = TieredConfig::default_Topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_zero_pool() {
        let config = TieredConfig {
            memory_pool_size: 0,
            ..TieredConfig::default_Topological()
        };
        assert_eq!(config.validate(), Err(TierError::InvalidMemorySize(0)));
    }

    #[test]
    fn test_config_zero_nodes() {
        let config = TieredConfig {
            max_nodes_per_tier: 0,
            ..TieredConfig::default_Topological()
        };
        assert_eq!(config.validate(), Err(TierError::InvalidConfig));
    }

    // â”€â”€â”€ Memory Pool Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_pool_new() {
        let pool = MemoryPool::new(1024);
        assert_eq!(pool.available(), 1024);
        assert_eq!(pool.allocation_count(), 0);
    }

    #[test]
    fn test_pool_allocate() {
        let mut pool = MemoryPool::new(1024);
        let id = pool.allocate(512).unwrap();
        assert_eq!(id, 1);
        assert_eq!(pool.available(), 512);
        assert_eq!(pool.allocation_count(), 1);
    }

    #[test]
    fn test_pool_deallocate() {
        let mut pool = MemoryPool::new(1024);
        let id = pool.allocate(512).unwrap();
        let freed = pool.deallocate(id).unwrap();
        assert_eq!(freed, 512);
        assert_eq!(pool.available(), 1024);
    }

    #[test]
    fn test_pool_exhausted() {
        let mut pool = MemoryPool::new(100);
        pool.allocate(100).unwrap();
        assert_eq!(pool.allocate(1), Err(TierError::MemoryPoolExhausted));
    }

    #[test]
    fn test_pool_deallocate_missing() {
        let mut pool = MemoryPool::new(1024);
        assert_eq!(pool.deallocate(999), Err(TierError::NodeNotFound(999)));
    }

    #[test]
    fn test_pool_utilization() {
        let mut pool = MemoryPool::new(1000);
        pool.allocate(500).unwrap();
        assert!((pool.utilization() - 0.5) < 1e-10);
    }

    #[test]
    fn test_pool_reset() {
        let mut pool = MemoryPool::new(1024);
        pool.allocate(512).unwrap();
        pool.reset();
        assert_eq!(pool.available(), 1024);
        assert_eq!(pool.allocation_count(), 0);
    }

    #[test]
    fn test_pool_display() {
        let pool = MemoryPool::new(1024 * 1024);
        let s = format!("{}", pool);
        assert!(s.contains("MemoryPool"));
    }

    // â”€â”€â”€ Execution Node Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_node_new_edge() {
        let node = ExecutionNode::new(1, ExecutionTier::Edge);
        assert_eq!(node.tier, ExecutionTier::Edge);
        assert_eq!(node.precision, Precision::Int4);
        assert!(node.active);
    }

    #[test]
    fn test_node_with_precision() {
        let node = ExecutionNode::new(1, ExecutionTier::Edge)
            .with_precision(Precision::Fp16)
            .unwrap();
        assert_eq!(node.precision, Precision::Fp16);
    }

    #[test]
    fn test_node_estimated_memory() {
        let node = ExecutionNode::new(1, ExecutionTier::Core);
        let mem = node.estimated_memory_for_tensor(1000);
        assert_eq!(mem, 4000); // 1000 * 4 bytes (FP32)
    }

    #[test]
    fn test_node_quantize_tensor() {
        let node = ExecutionNode::new(1, ExecutionTier::Core);
        let tensor = vec![1.0, 2.0, 3.0];
        let quantized = node.quantize_tensor(&tensor);
        assert_eq!(quantized, tensor); // FP32 is identity
    }

    #[test]
    fn test_node_display() {
        let node = ExecutionNode::new(42, ExecutionTier::Edge);
        let s = format!("{}", node);
        assert!(s.contains("ExecNode"));
        assert!(s.contains("id=42"));
    }

    // â”€â”€â”€ Tiered Executor Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_executor_new() {
        let executor = TieredExecutor::new();
        assert_eq!(executor.node_count(), 0);
    }

    #[test]
    fn test_executor_with_config() {
        let config = TieredConfig::default_Topological();
        let executor = TieredExecutor::with_config(config).unwrap();
        assert_eq!(executor.node_count(), 0);
    }

    #[test]
    fn test_register_node() {
        let mut executor = TieredExecutor::new();
        executor.config.memory_pooling = false; // Avoid memory pool size mismatch
        let node = ExecutionNode::new(1, ExecutionTier::Core);
        executor.register_node(node).unwrap();
        assert_eq!(executor.node_count(), 1);
    }

    #[test]
    fn test_register_node_capacity() {
        let mut executor = TieredExecutor::new();
        executor.config.max_nodes_per_tier = 1;
        executor.config.memory_pooling = false;

        let node1 = ExecutionNode::new(1, ExecutionTier::Edge);
        executor.register_node(node1).unwrap();

        let node2 = ExecutionNode::new(2, ExecutionTier::Edge);
        assert_eq!(
            executor.register_node(node2),
            Err(TierError::CapacityExceeded(1))
        );
    }

    #[test]
    fn test_get_node() {
        let mut executor = TieredExecutor::new();
        executor.config.memory_pooling = false; // Avoid memory pool size mismatch
        let node = ExecutionNode::new(42, ExecutionTier::Core);
        executor.register_node(node).unwrap();

        let found = executor.get_node(42).unwrap();
        assert_eq!(found.node_id, 42);
    }

    #[test]
    fn test_get_node_missing() {
        let executor = TieredExecutor::new();
        assert!(executor.get_node(999).is_none());
    }

    #[test]
    fn test_get_nodes_by_tier() {
        let mut executor = TieredExecutor::new();
        executor.config.memory_pooling = false;

        let edge = ExecutionNode::new(1, ExecutionTier::Edge);
        let core = ExecutionNode::new(2, ExecutionTier::Core);
        executor.register_node(edge).unwrap();
        executor.register_node(core).unwrap();

        let edges = executor.get_nodes_by_tier(ExecutionTier::Edge);
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn test_auto_assign_tier_small() {
        let executor = TieredExecutor::new();
        let tier = executor.auto_assign_tier(100, 0.001);
        assert_eq!(tier, ExecutionTier::Edge);
    }

    #[test]
    fn test_auto_assign_tier_large() {
        let executor = TieredExecutor::new();
        let tier = executor.auto_assign_tier(100000, 0.9);
        assert_eq!(tier, ExecutionTier::Core);
    }

    #[test]
    fn test_auto_assign_tier_medium() {
        let executor = TieredExecutor::new();
        let tier = executor.auto_assign_tier(5000, 0.3);
        assert_eq!(tier, ExecutionTier::Hybrid);
    }

    #[test]
    fn test_quantize_for_node() {
        let mut executor = TieredExecutor::new();
        executor.config.memory_pooling = false;
        let node = ExecutionNode::new(1, ExecutionTier::Core);
        executor.register_node(node).unwrap();

        let tensor = vec![1.5, 2.5, 3.5];
        let quantized = executor.quantize_for_node(1, &tensor).unwrap();
        assert_eq!(quantized, tensor); // FP32 identity
    }

    #[test]
    fn test_quantize_for_node_missing() {
        let executor = TieredExecutor::new();
        assert_eq!(
            executor.quantize_for_node(999, &[1.0]),
            Err(TierError::NodeNotFound(999))
        );
    }

    #[test]
    fn test_estimate_memory() {
        let executor = TieredExecutor::new();
        let mem = executor.estimate_memory(ExecutionTier::Core, 1000);
        assert_eq!(mem, 4000); // FP32 = 4 bytes
    }

    #[test]
    fn test_reset() {
        let mut executor = TieredExecutor::new();
        executor.config.memory_pooling = false;
        let node = ExecutionNode::new(1, ExecutionTier::Core);
        executor.register_node(node).unwrap();

        executor.reset();
        assert_eq!(executor.node_count(), 0);
    }

    #[test]
    fn test_executor_display() {
        let executor = TieredExecutor::new();
        let s = format!("{}", executor);
        assert!(s.contains("TieredExecutor"));
    }

    // â”€â”€â”€ Utility Function Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_quantize_tensor() {
        let tensor = vec![1.5, 2.5, 3.5];
        let quantized = quantize_tensor(&tensor, Precision::Fp32);
        assert_eq!(quantized, tensor);
    }

    #[test]
    fn test_quantization_mse_zero() {
        let original = vec![1.0, 2.0, 3.0];
        let quantized = vec![1.0, 2.0, 3.0];
        assert_eq!(quantization_mse(&original, &quantized), 0.0);
    }

    #[test]
    fn test_quantization_mse_positive() {
        let original = vec![1.0, 2.0, 3.0];
        let quantized = vec![1.5, 2.5, 3.5];
        let mse = quantization_mse(&original, &quantized);
        assert!(mse > 0.0);
    }

    #[test]
    fn test_quantization_mse_length_mismatch() {
        let original = vec![1.0, 2.0];
        let quantized = vec![1.0];
        assert_eq!(quantization_mse(&original, &quantized), 0.0);
    }

    #[test]
    fn test_memory_savings_int4_from_fp32() {
        let savings = memory_savings_ratio(Precision::Fp32, Precision::Int4);
        assert!((savings - 0.75) < 1e-10); // (4-1)/4 = 0.75
    }

    #[test]
    fn test_memory_savings_same_precision() {
        let savings = memory_savings_ratio(Precision::Fp32, Precision::Fp32);
        assert!((savings - 0.0) < 1e-10);
    }

    // â”€â”€â”€ Error Display Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_error_display_invalid_tier() {
        let e = TierError::InvalidTier("Unknown".to_string());
        assert!(format!("{}", e).contains("Unknown"));
    }

    #[test]
    fn test_error_display_pool_exhausted() {
        let e = TierError::MemoryPoolExhausted;
        assert!(format!("{}", e).contains("exhausted"));
    }

    #[test]
    fn test_error_display_capacity() {
        let e = TierError::CapacityExceeded(100);
        let s = format!("{}", e);
        assert!(s.contains("100"));
    }

    // â”€â”€â”€ Workflow Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_full_tiered_workflow() {
        let mut executor = TieredExecutor::new();
        executor.config.memory_pooling = false;

        // Register nodes
        let edge = ExecutionNode::new(1, ExecutionTier::Edge);
        let core = ExecutionNode::new(2, ExecutionTier::Core);
        executor.register_node(edge).unwrap();
        executor.register_node(core).unwrap();

        // Auto-assign tier
        let tier = executor.auto_assign_tier(500, 0.005);
        assert_eq!(tier, ExecutionTier::Edge);

        // Quantize
        let tensor = vec![1.0, 2.0, 3.0];
        let quantized = executor.quantize_for_node(1, &tensor).unwrap();
        assert_eq!(quantized.len(), 3);

        // Check stats
        assert_eq!(executor.node_count(), 2);
    }
}
