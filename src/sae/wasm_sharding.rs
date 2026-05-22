//! WASM Micro-Sharding — Tensor chunking for wasm32 peers (≤50MB chunks).
//!
//! Feature-gated behind `v2.1-wasm-micro-sharding`. Provides safe tensor
//! slicing via candle-core with stride/shape preservation for wasm32 peers
//! that have limited memory capacity.
//!
//! **Status:** Scaffold — safe slicing + shape preservation.
//! **License:** Apache 2.0 + Ethical Use Clause

use candle_core::{DType, Device, Tensor};
use thiserror::Error;

/// Maximum chunk size for wasm32 peers (50 MB).
const MAX_WASM_CHUNK_SIZE_MB: usize = 50;

/// Maximum tensor dimension count supported.
const MAX_DIMS: usize = 4;

/// Errors specific to WASM micro-sharding operations.
#[derive(Debug, Error)]
pub enum WasmShardError {
    #[error("Tensor operation failed: {0}")]
    TensorOp(#[from] candle_core::Error),

    #[error("Chunk size exceeds wasm32 limit: {0} MB > {1} MB")]
    ChunkTooLarge(usize, usize),

    #[error("Invalid tensor shape: {0}")]
    InvalidShape(String),

    #[error("Unsupported dimension count: {0} (max {1})")]
    DimsExceeded(usize, usize),

    #[error("Peer is not wasm32-capable: {0}")]
    NotWasmPeer(String),

    #[error("Shard index out of range: {0} >= {1}")]
    ShardIndexOutOfRange(usize, usize),
}

/// Describes the capability profile of a peer for WASM sharding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmPeerProfile {
    /// Peer identifier.
    pub peer_id: String,
    /// Target architecture (must be "wasm32" for sharding).
    pub arch: String,
    /// Available memory in MB.
    pub available_memory_mb: usize,
    /// Maximum chunk size this peer can handle (MB).
    pub max_chunk_mb: usize,
}

impl WasmPeerProfile {
    /// Create a new peer profile.
    pub fn new(peer_id: String, arch: String, available_memory_mb: usize) -> Self {
        let max_chunk_mb = std::cmp::min(available_memory_mb / 2, MAX_WASM_CHUNK_SIZE_MB);
        Self {
            peer_id,
            arch,
            available_memory_mb,
            max_chunk_mb,
        }
    }

    /// Check if this peer is wasm32-capable.
    pub fn is_wasm_peer(&self) -> bool {
        self.arch == "wasm32"
    }

    /// Check if this peer can handle a chunk of the given size.
    pub fn can_handle_chunk(&self, chunk_size_mb: usize) -> bool {
        self.is_wasm_peer() && chunk_size_mb <= self.max_chunk_mb
    }
}

/// A single shard of a tensor, safe for wasm32 transmission.
#[derive(Debug, Clone)]
pub struct TensorShard {
    /// Shard index within the original tensor.
    pub shard_index: usize,
    /// Total number of shards.
    pub total_shards: usize,
    /// Shard data as f32 values (flattened).
    pub data: Vec<f32>,
    /// Original shape of this shard.
    pub shape: Vec<usize>,
    /// Stride information for reconstruction.
    pub stride: Vec<usize>,
    /// Offset in the original tensor (flattened index).
    pub offset: usize,
    /// Size in MB.
    pub size_mb: usize,
}

impl TensorShard {
    /// Create a new tensor shard.
    pub fn new(
        shard_index: usize,
        total_shards: usize,
        data: Vec<f32>,
        shape: Vec<usize>,
        stride: Vec<usize>,
        offset: usize,
    ) -> Self {
        let size_mb = data.len() * std::mem::size_of::<f32>() / (1024 * 1024);
        Self {
            shard_index,
            total_shards,
            data,
            shape,
            stride,
            offset,
            size_mb,
        }
    }

    /// Convert shard back to a candle-core Tensor.
    pub fn to_tensor(&self, device: &Device) -> Result<Tensor, WasmShardError> {
        let tensor: Tensor = match self.shape.len() {
            1 => Tensor::from_vec(self.data.clone(), self.shape[0], device)?,
            2 => Tensor::from_slice(&self.data, (self.shape[0], self.shape[1]), device)?,
            3 => Tensor::from_slice(
                &self.data,
                (self.shape[0], self.shape[1], self.shape[2]),
                device,
            )?,
            4 => Tensor::from_slice(
                &self.data,
                (self.shape[0], self.shape[1], self.shape[2], self.shape[3]),
                device,
            )?,
            _ => {
                return Err(WasmShardError::InvalidShape(format!(
                    "Unsupported shape dimension count: {}",
                    self.shape.len()
                )))
            }
        };
        Ok(tensor)
    }
}

/// Result of sharding a tensor for wasm32 peers.
#[derive(Debug)]
pub struct ShardedTensor {
    /// Original tensor shape.
    pub original_shape: Vec<usize>,
    /// Original tensor dtype.
    pub dtype: DType,
    /// Shards.
    pub shards: Vec<TensorShard>,
    /// Total size in MB.
    pub total_size_mb: usize,
}

impl ShardedTensor {
    /// Get a specific shard by index.
    pub fn get_shard(&self, index: usize) -> Result<&TensorShard, WasmShardError> {
        match self.shards.get(index) {
            Some(shard) => Ok(shard),
            None => Err(WasmShardError::ShardIndexOutOfRange(
                index,
                self.shards.len(),
            )),
        }
    }

    /// Filter shards that a specific peer can handle.
    pub fn filter_for_peer(&self, peer: &WasmPeerProfile) -> Vec<&TensorShard> {
        if !peer.is_wasm_peer() {
            return Vec::new();
        }
        self.shards
            .iter()
            .filter(|s| s.size_mb <= peer.max_chunk_mb)
            .collect()
    }
}

/// Detect if a peer is wasm32-capable based on user agent or capability string.
pub fn detect_wasm_peer(user_agent: &str) -> bool {
    let ua = user_agent.to_lowercase();
    ua.contains("wasm") || ua.contains("webassembly") || ua.contains("browser-node")
}

/// Estimate the memory size of a tensor in MB.
fn estimate_tensor_size_mb(shape: &[usize], dtype: DType) -> usize {
    let elem_count: usize = shape.iter().product();
    let bytes_per_elem = match dtype {
        DType::F32 => std::mem::size_of::<f32>(),
        DType::F64 => std::mem::size_of::<f64>(),
        DType::U8 => std::mem::size_of::<u8>(),
        DType::I64 => std::mem::size_of::<i64>(),
        DType::BF16 => 2,
        DType::F16 => 2,
        DType::U32 => std::mem::size_of::<u32>(),
    };
    elem_count * bytes_per_elem / (1024 * 1024)
}

/// Calculate the optimal chunk size along the first dimension.
fn calculate_chunk_size(shape: &[usize], dtype: DType, max_chunk_mb: usize) -> usize {
    if shape.is_empty() {
        return 1;
    }

    let total_size_mb = estimate_tensor_size_mb(shape, dtype);
    if total_size_mb <= max_chunk_mb {
        return shape[0]; // No sharding needed.
    }

    // Calculate elements per MB.
    let _elem_count: usize = shape.iter().product();
    let bytes_per_elem = match dtype {
        DType::F32 => std::mem::size_of::<f32>(),
        DType::F64 => std::mem::size_of::<f64>(),
        DType::U8 => std::mem::size_of::<u8>(),
        DType::I64 => std::mem::size_of::<i64>(),
        DType::BF16 => 2,
        DType::F16 => 2,
        DType::U32 => std::mem::size_of::<u32>(),
    };

    let max_elements = max_chunk_mb * 1024 * 1024 / bytes_per_elem;

    // Calculate trailing dimension product (all dims except first).
    let trailing: usize = shape[1..].iter().product();
    if trailing == 0 {
        return 1;
    }

    // Chunk size along first dimension.
    let chunk_size = std::cmp::max(1, max_elements / trailing);
    std::cmp::min(chunk_size, shape[0])
}

/// Shard a tensor for wasm32 peers, ensuring each chunk is ≤50MB.
pub fn shard_tensor_for_wasm(tensor: &Tensor) -> Result<ShardedTensor, WasmShardError> {
    let shape: Vec<usize> = tensor.shape().dims().iter().map(|d| *d as usize).collect();
    let dtype = tensor.dtype();
    let _device = tensor.device();

    // Validate dimension count.
    if shape.len() > MAX_DIMS {
        return Err(WasmShardError::DimsExceeded(shape.len(), MAX_DIMS));
    }

    let total_size_mb = estimate_tensor_size_mb(&shape, dtype);

    // If tensor fits in one chunk, return as single shard.
    if total_size_mb <= MAX_WASM_CHUNK_SIZE_MB {
        let data = extract_tensor_data(tensor)?;
        let stride = compute_stride(&shape);
        let shard = TensorShard::new(0, 1, data, shape.clone(), stride, 0);
        return Ok(ShardedTensor {
            original_shape: shape,
            dtype,
            shards: vec![shard],
            total_size_mb,
        });
    }

    // Calculate chunk size along first dimension.
    let chunk_size = calculate_chunk_size(&shape, dtype, MAX_WASM_CHUNK_SIZE_MB);

    // Shard along first dimension using slice + squeeze.
    let mut shards = Vec::new();
    let dim0 = shape[0];
    let mut offset = 0usize;

    for i in (0..dim0).step_by(chunk_size) {
        let end = std::cmp::min(i + chunk_size, dim0);
        let slice_len = end - i;

        // Slice the tensor along first dimension.
        let sliced = tensor.narrow(0, i, slice_len)?;
        let shard_shape: Vec<usize> = sliced.shape().dims().iter().map(|d| *d as usize).collect();
        let shard_data = extract_tensor_data(&sliced)?;
        let shard_stride = compute_stride(&shard_shape);

        let shard = TensorShard::new(
            shards.len(),
            0, // Will be set after loop.
            shard_data,
            shard_shape,
            shard_stride,
            offset,
        );
        offset += slice_len;
        shards.push(shard);
    }

    // Set total_shards for all shards.
    let total_shards = shards.len();
    for shard in shards.iter_mut() {
        shard.total_shards = total_shards;
    }

    Ok(ShardedTensor {
        original_shape: shape,
        dtype,
        shards,
        total_size_mb,
    })
}

/// Extract flattened f32 data from a tensor.
fn extract_tensor_data(tensor: &Tensor) -> Result<Vec<f32>, WasmShardError> {
    // Convert to f32 if needed, then flatten.
    let f32_tensor = tensor.to_dtype(DType::F32)?;
    let flattened = f32_tensor.flatten_all()?;
    let data: Vec<f32> = flattened.to_vec1()?;
    Ok(data)
}

/// Compute stride for a given shape (row-major/C-contiguous).
fn compute_stride(shape: &[usize]) -> Vec<usize> {
    let mut stride = vec![1usize; shape.len()];
    for i in (0..shape.len() - 1).rev() {
        stride[i] = stride[i + 1] * shape[i + 1];
    }
    stride
}

/// Reconstruct a tensor from shards.
pub fn reconstruct_tensor(
    sharded: &ShardedTensor,
    device: &Device,
) -> Result<Tensor, WasmShardError> {
    if sharded.shards.is_empty() {
        return Err(WasmShardError::InvalidShape(
            "No shards to reconstruct".to_string(),
        ));
    }

    // Sort shards by index.
    let mut sorted_shards = sharded.shards.clone();
    sorted_shards.sort_by_key(|s| s.shard_index);

    // Concatenate along first dimension.
    let mut tensors: Vec<Tensor> = Vec::new();
    for shard in &sorted_shards {
        tensors.push(shard.to_tensor(device)?);
    }

    if tensors.len() == 1 {
        Ok(tensors.into_iter().next().unwrap())
    } else {
        Tensor::cat(&tensors, 0).map_err(WasmShardError::TensorOp)
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_wasm_peer_wasm_agent() {
        assert!(detect_wasm_peer("Mozilla/5.0 (WASM)"));
    }

    #[test]
    fn test_detect_wasm_peer_browser_node() {
        assert!(detect_wasm_peer("ed2kIA browser-node/1.0"));
    }

    #[test]
    fn test_detect_wasm_peer_webassembly() {
        assert!(detect_wasm_peer("WebAssembly/1.0"));
    }

    #[test]
    fn test_detect_wasm_peer_not_wasm() {
        assert!(!detect_wasm_peer("Mozilla/5.0 (Windows NT 10.0)"));
    }

    #[test]
    fn test_wasm_peer_profile_new() {
        let profile = WasmPeerProfile::new("peer-1".to_string(), "wasm32".to_string(), 128);
        assert_eq!(profile.peer_id, "peer-1");
        assert_eq!(profile.arch, "wasm32");
        assert_eq!(profile.available_memory_mb, 128);
        assert_eq!(profile.max_chunk_mb, 50); // min(128/2, 50) = 50
    }

    #[test]
    fn test_wasm_peer_profile_limited_memory() {
        let profile = WasmPeerProfile::new("peer-2".to_string(), "wasm32".to_string(), 40);
        assert_eq!(profile.max_chunk_mb, 20); // min(40/2, 50) = 20
    }

    #[test]
    fn test_wasm_peer_profile_is_wasm() {
        let wasm_profile = WasmPeerProfile::new("w".to_string(), "wasm32".to_string(), 64);
        assert!(wasm_profile.is_wasm_peer());

        let native_profile = WasmPeerProfile::new("n".to_string(), "x86_64".to_string(), 1024);
        assert!(!native_profile.is_wasm_peer());
    }

    #[test]
    fn test_wasm_peer_profile_can_handle_chunk() {
        let profile = WasmPeerProfile::new("p".to_string(), "wasm32".to_string(), 128);
        assert!(profile.can_handle_chunk(50));
        assert!(profile.can_handle_chunk(25));
        assert!(!profile.can_handle_chunk(51));
    }

    #[test]
    fn test_estimate_tensor_size_mb_f32() {
        // 1000 x 1000 f32 = 3,906,250 bytes ≈ 3 MB (integer division)
        let size = estimate_tensor_size_mb(&[1000, 1000], DType::F32);
        assert!(size >= 3 && size <= 4);
    }

    #[test]
    fn test_estimate_tensor_size_mb_f64() {
        // 1000 x 1000 f64 = 7,812,500 bytes ≈ 7 MB (integer division)
        let size = estimate_tensor_size_mb(&[1000, 1000], DType::F64);
        assert!(size >= 7 && size <= 8);
    }

    #[test]
    fn test_compute_stride_1d() {
        let stride = compute_stride(&[10]);
        assert_eq!(stride, vec![1]);
    }

    #[test]
    fn test_compute_stride_2d() {
        let stride = compute_stride(&[3, 4]);
        assert_eq!(stride, vec![4, 1]);
    }

    #[test]
    fn test_compute_stride_3d() {
        let stride = compute_stride(&[2, 3, 4]);
        assert_eq!(stride, vec![12, 4, 1]);
    }

    #[test]
    fn test_shard_small_tensor() {
        let device = Device::Cpu;
        let data: Vec<f32> = (0..100).map(|x| x as f32).collect();
        let tensor = Tensor::from_vec(data, 100, &device).unwrap();

        let sharded = shard_tensor_for_wasm(&tensor).unwrap();
        assert_eq!(sharded.shards.len(), 1);
        assert_eq!(sharded.original_shape, vec![100]);
        assert_eq!(sharded.shards[0].shard_index, 0);
        assert_eq!(sharded.shards[0].total_shards, 1);
    }

    #[test]
    fn test_shard_tensor_2d() {
        let device = Device::Cpu;
        let data: Vec<f32> = (0..1000).map(|x| x as f32).collect();
        let tensor = Tensor::from_slice(&data, (10, 100), &device).unwrap();

        let sharded = shard_tensor_for_wasm(&tensor).unwrap();
        assert_eq!(sharded.original_shape, vec![10, 100]);
        assert!(!sharded.shards.is_empty());
    }

    #[test]
    fn test_sharded_tensor_get_shard() {
        let device = Device::Cpu;
        let data: Vec<f32> = (0..100).map(|x| x as f32).collect();
        let tensor = Tensor::from_vec(data, 100, &device).unwrap();

        let sharded = shard_tensor_for_wasm(&tensor).unwrap();
        let shard = sharded.get_shard(0).unwrap();
        assert_eq!(shard.shard_index, 0);
    }

    #[test]
    fn test_sharded_tensor_get_shard_out_of_range() {
        let device = Device::Cpu;
        let data: Vec<f32> = (0..100).map(|x| x as f32).collect();
        let tensor = Tensor::from_vec(data, 100, &device).unwrap();

        let sharded = shard_tensor_for_wasm(&tensor).unwrap();
        let result = sharded.get_shard(5);
        assert!(result.is_err());
    }

    #[test]
    fn test_sharded_tensor_filter_for_peer() {
        let device = Device::Cpu;
        let data: Vec<f32> = (0..100).map(|x| x as f32).collect();
        let tensor = Tensor::from_vec(data, 100, &device).unwrap();

        let sharded = shard_tensor_for_wasm(&tensor).unwrap();

        let wasm_peer = WasmPeerProfile::new("w".to_string(), "wasm32".to_string(), 128);
        let filtered = sharded.filter_for_peer(&wasm_peer);
        assert_eq!(filtered.len(), 1);

        let native_peer = WasmPeerProfile::new("n".to_string(), "x86_64".to_string(), 1024);
        let filtered = sharded.filter_for_peer(&native_peer);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_tensor_shard_new() {
        let shard = TensorShard::new(0, 1, vec![1.0, 2.0, 3.0], vec![3], vec![1], 0);
        assert_eq!(shard.shard_index, 0);
        assert_eq!(shard.total_shards, 1);
        assert_eq!(shard.data.len(), 3);
        assert_eq!(shard.shape, vec![3]);
        assert_eq!(shard.offset, 0);
    }

    #[test]
    fn test_reconstruct_single_shard() {
        let device = Device::Cpu;
        let data: Vec<f32> = (0..100).map(|x| x as f32).collect();
        let tensor = Tensor::from_vec(data.clone(), 100, &device).unwrap();

        let sharded = shard_tensor_for_wasm(&tensor).unwrap();
        let reconstructed = reconstruct_tensor(&sharded, &device).unwrap();

        let reconstructed_dims: Vec<usize> = reconstructed
            .shape()
            .dims()
            .iter()
            .map(|d| *d as usize)
            .collect();
        assert_eq!(reconstructed_dims, vec![100]);
    }

    #[test]
    fn test_reconstruct_empty_shards() {
        let sharded = ShardedTensor {
            original_shape: vec![],
            dtype: DType::F32,
            shards: vec![],
            total_size_mb: 0,
        };
        let result = reconstruct_tensor(&sharded, &Device::Cpu);
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_chunk_size_small_tensor() {
        // Small tensor should return full first dim.
        let chunk = calculate_chunk_size(&[100], DType::F32, MAX_WASM_CHUNK_SIZE_MB);
        assert_eq!(chunk, 100);
    }

    #[test]
    fn test_error_display() {
        let err = WasmShardError::ChunkTooLarge(75, 50);
        let msg = format!("{}", err);
        assert!(msg.contains("75"));
        assert!(msg.contains("50"));
    }
}
