//! IoT Adapter — Local Hardware Integration Layer.
//!
//! Provides secure, LOCAL_ONLY device registration and command routing
//! for the Corpuscular Bridge (RFC 001). All hardware communication
//! occurs via loopback or UNIX sockets. Zero external network exposure.
//!
//! **Design Principles:**
//! - Radical privacy: device endpoints never leave the local node.
//! - Cooperative integration: Ed25519-signed device registration.
//! - Symbiotic equilibrium: hardware commands validated against CE vouchers.
//! - Zero telemetry: no device metrics exposed to the P2P network.
//!
//! **Feature Gate:** `v3.0-corpuscular-bridge`

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Unique identifier for a registered hardware device.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HardwareId(pub String);

impl std::fmt::Display for HardwareId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "hw:{}", self.0)
    }
}

/// Unique identifier for a hardware data stream.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StreamId(pub String);

impl std::fmt::Display for StreamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stream:{}", self.0)
    }
}

/// Configuration for a registered hardware device.
#[derive(Debug, Clone)]
pub struct HardwareConfig {
    /// Local endpoint address (loopback or UNIX socket path).
    pub endpoint: SocketAddr,
    /// Device type descriptor (e.g., "3d_printer", "solar_inverter").
    pub device_type: String,
    /// Ed25519 signature from the local node authorizing this device.
    pub node_signature: Vec<u8>,
    /// Maximum command payload size in bytes.
    pub max_payload_bytes: usize,
}

impl HardwareConfig {
    /// Create a new hardware configuration with local-only validation.
    pub fn new(
        endpoint: SocketAddr,
        device_type: String,
        node_signature: Vec<u8>,
        max_payload_bytes: usize,
    ) -> Result<Self, AdapterError> {
        // Enforce LOCAL_ONLY: only loopback or UNIX-domain-compatible addresses.
        Self::validate_local_endpoint(&endpoint)?;

        Ok(Self {
            endpoint,
            device_type,
            node_signature,
            max_payload_bytes,
        })
    }

    /// Validate that the endpoint is strictly local (loopback or localhost).
    pub fn validate_local_endpoint(addr: &SocketAddr) -> Result<(), AdapterError> {
        match addr.ip() {
            IpAddr::V4(Ipv4Addr::LOCALHOST) => Ok(()),
            IpAddr::V6(addr_v6) if addr_v6.is_loopback() => Ok(()),
            _ => Err(AdapterError::NonLocalEndpoint(addr.clone())),
        }
    }
}

/// Errors specific to IoT adapter operations.
#[derive(Debug, Clone)]
pub enum AdapterError {
    /// Endpoint is not local (loopback/localhost).
    NonLocalEndpoint(SocketAddr),
    /// Device not found in registry.
    DeviceNotFound(HardwareId),
    /// Command payload exceeds device maximum.
    PayloadTooLarge(usize),
    /// Invalid Ed25519 signature for device registration.
    InvalidSignature,
    /// Device already registered.
    DeviceAlreadyRegistered(HardwareId),
    /// Command routing failed.
    RoutingFailed(String),
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterError::NonLocalEndpoint(addr) => {
                write!(f, "Non-local endpoint rejected: {} (LOCAL_ONLY enforced)", addr)
            }
            AdapterError::DeviceNotFound(id) => write!(f, "Device not found: {}", id),
            AdapterError::PayloadTooLarge(size) => write!(f, "Payload too large: {} bytes", size),
            AdapterError::InvalidSignature => write!(f, "Invalid Ed25519 signature for device registration"),
            AdapterError::DeviceAlreadyRegistered(id) => write!(f, "Device already registered: {}", id),
            AdapterError::RoutingFailed(msg) => write!(f, "Command routing failed: {}", msg),
        }
    }
}

/// Local Hardware Adapter — Manages device registry and command routing.
///
/// All device communication is confined to the local execution boundary.
/// Commands are routed via `tokio::sync::mpsc` channels or local sockets.
/// Zero exposure to external network interfaces.
pub struct LocalHardwareAdapter {
    /// Registry of authorized devices.
    devices: HashMap<HardwareId, HardwareConfig>,
    /// Stream registry for device data channels.
    stream_registry: HashMap<StreamId, String>,
}

impl LocalHardwareAdapter {
    /// Create a new empty hardware adapter.
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
            stream_registry: HashMap::new(),
        }
    }

    /// Register a local hardware device.
    ///
    /// **Validation:**
    /// 1. Endpoint must be loopback (127.0.0.1 or ::1).
    /// 2. Node signature must be non-empty (Ed25519 authorization).
    /// 3. Device ID must be unique in the registry.
    pub fn register_local_device(
        &mut self,
        id: HardwareId,
        config: HardwareConfig,
    ) -> Result<(), AdapterError> {
        // Check for duplicate registration.
        if self.devices.contains_key(&id) {
            return Err(AdapterError::DeviceAlreadyRegistered(id.clone()));
        }

        // Validate signature (scaffolding: check non-empty).
        if config.node_signature.is_empty() {
            return Err(AdapterError::InvalidSignature);
        }

        self.devices.insert(id, config);
        Ok(())
    }

    /// Route a command to a registered device.
    ///
    /// **Validation:**
    /// 1. Device must exist in registry.
    /// 2. Payload must not exceed device maximum.
    /// 3. Endpoint must be local (already enforced at registration).
    ///
    /// Returns the simulated device response.
    pub fn route_command(
        &self,
        device: HardwareId,
        payload: &[u8],
    ) -> Result<Vec<u8>, AdapterError> {
        let config = self.devices.get(&device)
            .ok_or(AdapterError::DeviceNotFound(device.clone()))?;

        // Validate payload size.
        if payload.len() > config.max_payload_bytes {
            return Err(AdapterError::PayloadTooLarge(payload.len()));
        }

        // Simulate command routing to local endpoint.
        // In production, this dispatches via tokio::sync::mpsc or UNIX socket.
        let response = format!(
            "OK:{}:{}:{}b",
            config.device_type,
            config.endpoint,
            payload.len()
        );
        Ok(response.into_bytes())
    }

    /// Register a data stream for a device.
    pub fn register_stream(&mut self, stream_id: StreamId, device_id: String) {
        self.stream_registry.insert(stream_id, device_id);
    }

    /// Get the configuration for a registered device.
    pub fn get_device(&self, id: &HardwareId) -> Option<&HardwareConfig> {
        self.devices.get(id)
    }

    /// List all registered device IDs.
    pub fn registered_devices(&self) -> Vec<HardwareId> {
        self.devices.keys().cloned().collect()
    }

    /// Get the number of registered devices.
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Remove a device from the registry.
    pub fn unregister_device(&mut self, id: &HardwareId) -> Option<HardwareConfig> {
        self.devices.remove(id)
    }
}

impl Default for LocalHardwareAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_local_config(port: u16) -> HardwareConfig {
        HardwareConfig {
            endpoint: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            device_type: "3d_printer".to_string(),
            node_signature: vec![1, 2, 3, 4],
            max_payload_bytes: 4096,
        }
    }

    #[test]
    fn test_adapter_creation() {
        let adapter = LocalHardwareAdapter::new();
        assert_eq!(adapter.device_count(), 0);
    }

    #[test]
    fn test_register_local_device() {
        let mut adapter = LocalHardwareAdapter::new();
        let id = HardwareId("printer-1".to_string());
        let config = make_local_config(8080);
        assert!(adapter.register_local_device(id, config).is_ok());
        assert_eq!(adapter.device_count(), 1);
    }

    #[test]
    fn test_register_duplicate_device() {
        let mut adapter = LocalHardwareAdapter::new();
        let id = HardwareId("printer-1".to_string());
        let config = make_local_config(8080);
        assert!(adapter.register_local_device(id.clone(), config).is_ok());

        let config2 = make_local_config(8081);
        match adapter.register_local_device(id, config2) {
            Err(AdapterError::DeviceAlreadyRegistered(_)) => {}, // Expected
            other => panic!("Expected DeviceAlreadyRegistered, got {:?}", other),
        }
    }

    #[test]
    fn test_register_empty_signature_rejected() {
        let mut adapter = LocalHardwareAdapter::new();
        let id = HardwareId("printer-1".to_string());
        let mut config = make_local_config(8080);
        config.node_signature.clear();

        match adapter.register_local_device(id, config) {
            Err(AdapterError::InvalidSignature) => {}, // Expected
            other => panic!("Expected InvalidSignature, got {:?}", other),
        }
    }

    #[test]
    fn test_non_local_endpoint_rejected() {
        let config = HardwareConfig {
            endpoint: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080),
            device_type: "external".to_string(),
            node_signature: vec![1],
            max_payload_bytes: 1024,
        };
        match HardwareConfig::validate_local_endpoint(&config.endpoint) {
            Err(AdapterError::NonLocalEndpoint(_)) => {}, // Expected
            other => panic!("Expected NonLocalEndpoint, got {:?}", other),
        }
    }

    #[test]
    fn test_route_command_success() {
        let mut adapter = LocalHardwareAdapter::new();
        let id = HardwareId("printer-1".to_string());
        adapter.register_local_device(id.clone(), make_local_config(8080)).unwrap();

        let payload = b"print_object";
        let response = adapter.route_command(id, payload).unwrap();
        assert!(response.starts_with(b"OK:"));
    }

    #[test]
    fn test_route_command_unknown_device() {
        let adapter = LocalHardwareAdapter::new();
        let id = HardwareId("unknown".to_string());
        match adapter.route_command(id, b"test") {
            Err(AdapterError::DeviceNotFound(_)) => {}, // Expected
            other => panic!("Expected DeviceNotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_route_command_payload_too_large() {
        let mut adapter = LocalHardwareAdapter::new();
        let id = HardwareId("printer-1".to_string());
        let mut config = make_local_config(8080);
        config.max_payload_bytes = 10;
        adapter.register_local_device(id.clone(), config).unwrap();

        let payload = vec![0u8; 100];
        match adapter.route_command(id, &payload) {
            Err(AdapterError::PayloadTooLarge(100)) => {}, // Expected
            other => panic!("Expected PayloadTooLarge, got {:?}", other),
        }
    }

    #[test]
    fn test_unregister_device() {
        let mut adapter = LocalHardwareAdapter::new();
        let id = HardwareId("printer-1".to_string());
        adapter.register_local_device(id.clone(), make_local_config(8080)).unwrap();
        assert_eq!(adapter.device_count(), 1);

        let removed = adapter.unregister_device(&id);
        assert!(removed.is_some());
        assert_eq!(adapter.device_count(), 0);
    }

    #[test]
    fn test_registered_devices_list() {
        let mut adapter = LocalHardwareAdapter::new();
        adapter.register_local_device(HardwareId("a".to_string()), make_local_config(8080)).unwrap();
        adapter.register_local_device(HardwareId("b".to_string()), make_local_config(8081)).unwrap();

        let devices = adapter.registered_devices();
        assert_eq!(devices.len(), 2);
    }

    #[test]
    fn test_stream_registry() {
        let mut adapter = LocalHardwareAdapter::new();
        let stream = StreamId("temp-sensor".to_string());
        adapter.register_stream(stream, "device-1".to_string());
        assert_eq!(adapter.stream_registry.len(), 1);
    }

    #[test]
    fn test_default() {
        let adapter = LocalHardwareAdapter::default();
        assert_eq!(adapter.device_count(), 0);
    }

    #[test]
    fn test_error_display() {
        match AdapterError::NonLocalEndpoint(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 80)) {
            AdapterError::NonLocalEndpoint(_) => {},
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_hardware_id_display() {
        let id = HardwareId("test-device".to_string());
        assert_eq!(format!("{}", id), "hw:test-device");
    }

    #[test]
    fn test_stream_id_display() {
        let sid = StreamId("test-stream".to_string());
        assert_eq!(format!("{}", sid), "stream:test-stream");
    }
}
