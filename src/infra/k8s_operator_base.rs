//! K8s Operator Base v1 — Kubernetes operator foundation for ed2kIA v2.0.
//!
//! Defines CRDs (ed2kIA Node, Lease, SteeringConfig), mock reconciliation,
//! and YAML/JSON serialization tests for schema validation.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │           K8s Operator (ed2kIA)              │
//! │  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
//! │  │   Node   │  │  Lease   │  │ Steering │  │
//! │  │  CRD     │  │   CRD    │  │   CRD    │  │
//! │  └──────────┘  └──────────┘  └──────────┘  │
//! │  ┌────────────────────────────────────────┐ │
//! │  │         Reconciliation Loop            │ │
//! │  │  Watch → Reconcile → Update Status     │ │
//! │  └────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────┘
//! ```

mod internal {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    /// K8s operator error
    #[derive(Debug, Clone, PartialEq)]
    pub enum K8sError {
        /// Invalid CRD schema
        InvalidSchema(String),
        /// Resource not found
        NotFound(String),
        /// Reconciliation failed
        ReconciliationFailed(String),
        /// Serialization error
        Serialization(String),
    }

    impl std::fmt::Display for K8sError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                K8sError::InvalidSchema(msg) => write!(f, "Invalid schema: {}", msg),
                K8sError::NotFound(msg) => write!(f, "Not found: {}", msg),
                K8sError::ReconciliationFailed(msg) => write!(f, "Reconciliation failed: {}", msg),
                K8sError::Serialization(msg) => write!(f, "Serialization: {}", msg),
            }
        }
    }

    /// ed2kIA Node CRD spec
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NodeSpec {
        /// Node replica count
        pub replicas: i32,
        /// Node image
        pub image: String,
        /// Resource limits
        pub cpu_limit: String,
        pub memory_limit: String,
        /// Node port
        pub port: u16,
    }

    /// ed2kIA Node CRD status
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NodeStatus {
        pub ready: bool,
        pub current_replicas: i32,
        pub desired_replicas: i32,
        pub conditions: Vec<String>,
    }

    /// ed2kIA Node CRD
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NodeCRD {
        pub api_version: String,
        pub kind: String,
        pub metadata: ResourceMetadata,
        pub spec: NodeSpec,
        pub status: Option<NodeStatus>,
    }

    /// ed2kIA Lease CRD spec
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LeaseSpec {
        /// Lease duration in seconds
        pub duration_seconds: i64,
        /// Renew deadline in seconds
        pub renew_deadline_seconds: i64,
        /// Retry period in seconds
        pub retry_period_seconds: i64,
        /// Holder identity
        pub holder_identity: String,
    }

    /// ed2kIA Lease CRD status
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LeaseStatus {
        pub active: bool,
        pub holder: String,
        pub expires_at: Option<String>,
    }

    /// ed2kIA Lease CRD
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LeaseCRD {
        pub api_version: String,
        pub kind: String,
        pub metadata: ResourceMetadata,
        pub spec: LeaseSpec,
        pub status: Option<LeaseStatus>,
    }

    /// SteeringConfig CRD spec
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SteeringConfigSpec {
        /// Empathy level (0.0 — 1.0)
        pub empathy: f32,
        /// Creativity level (0.0 — 1.0)
        pub creativity: f32,
        /// Safety level (0.0 — 1.0)
        pub safety: f32,
        /// Target nodes
        pub target_nodes: Vec<String>,
    }

    /// SteeringConfig CRD status
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SteeringConfigStatus {
        pub applied: bool,
        pub applied_to: Vec<String>,
        pub last_applied: Option<String>,
    }

    /// SteeringConfig CRD
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SteeringConfigCRD {
        pub api_version: String,
        pub kind: String,
        pub metadata: ResourceMetadata,
        pub spec: SteeringConfigSpec,
        pub status: Option<SteeringConfigStatus>,
    }

    /// Common resource metadata
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ResourceMetadata {
        pub name: String,
        pub namespace: String,
        pub labels: HashMap<String, String>,
        pub annotations: HashMap<String, String>,
    }

    /// Reconciliation result
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ReconcileResult {
        pub resource_name: String,
        pub resource_kind: String,
        pub success: bool,
        pub message: String,
    }

    /// Mock K8s operator
    pub struct K8sOperator {
        nodes: HashMap<String, NodeCRD>,
        leases: HashMap<String, LeaseCRD>,
        steering_configs: HashMap<String, SteeringConfigCRD>,
        reconcile_log: Vec<ReconcileResult>,
    }

    impl K8sOperator {
        pub fn new() -> Self {
            Self {
                nodes: HashMap::new(),
                leases: HashMap::new(),
                steering_configs: HashMap::new(),
                reconcile_log: Vec::new(),
            }
        }

        pub fn add_node(&mut self, node: NodeCRD) -> Result<(), K8sError> {
            if node.spec.replicas < 0 {
                return Err(K8sError::InvalidSchema("Replicas cannot be negative".to_string()));
            }
            if node.spec.image.is_empty() {
                return Err(K8sError::InvalidSchema("Image cannot be empty".to_string()));
            }
            let key = format!("{}/{}", node.metadata.namespace, node.metadata.name);
            self.nodes.insert(key, node);
            Ok(())
        }

        pub fn add_lease(&mut self, lease: LeaseCRD) -> Result<(), K8sError> {
            if lease.spec.duration_seconds <= 0 {
                return Err(K8sError::InvalidSchema("Duration must be > 0".to_string()));
            }
            let key = format!("{}/{}", lease.metadata.namespace, lease.metadata.name);
            self.leases.insert(key, lease);
            Ok(())
        }

        pub fn add_steering_config(&mut self, config: SteeringConfigCRD) -> Result<(), K8sError> {
            if config.spec.empathy < 0.0 || config.spec.empathy > 1.0 {
                return Err(K8sError::InvalidSchema("Empathy must be 0.0-1.0".to_string()));
            }
            if config.spec.creativity < 0.0 || config.spec.creativity > 1.0 {
                return Err(K8sError::InvalidSchema("Creativity must be 0.0-1.0".to_string()));
            }
            if config.spec.safety < 0.0 || config.spec.safety > 1.0 {
                return Err(K8sError::InvalidSchema("Safety must be 0.0-1.0".to_string()));
            }
            let key = format!("{}/{}", config.metadata.namespace, config.metadata.name);
            self.steering_configs.insert(key, config);
            Ok(())
        }

        /// Mock reconciliation loop
        pub fn reconcile_all(&mut self) -> Vec<ReconcileResult> {
            let mut results = Vec::new();

            // Reconcile nodes
            for (key, node) in &self.nodes {
                let result = ReconcileResult {
                    resource_name: key.clone(),
                    resource_kind: "Node".to_string(),
                    success: true,
                    message: format!("Reconciled {} with {} replicas", node.metadata.name, node.spec.replicas),
                };
                results.push(result);
            }

            // Reconcile leases
            for (key, lease) in &self.leases {
                let result = ReconcileResult {
                    resource_name: key.clone(),
                    resource_kind: "Lease".to_string(),
                    success: true,
                    message: format!("Reconciled lease held by {}", lease.spec.holder_identity),
                };
                results.push(result);
            }

            // Reconcile steering configs
            for (key, config) in &self.steering_configs {
                let result = ReconcileResult {
                    resource_name: key.clone(),
                    resource_kind: "SteeringConfig".to_string(),
                    success: true,
                    message: format!("Applied steering to {} nodes", config.spec.target_nodes.len()),
                };
                results.push(result);
            }

            self.reconcile_log = results.clone();
            results
        }

        pub fn node_count(&self) -> usize {
            self.nodes.len()
        }

        pub fn lease_count(&self) -> usize {
            self.leases.len()
        }

        pub fn steering_config_count(&self) -> usize {
            self.steering_configs.len()
        }

        pub fn reconcile_log(&self) -> &[ReconcileResult] {
            &self.reconcile_log
        }
    }

    impl Default for K8sOperator {
        fn default() -> Self {
            Self::new()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn make_metadata(name: &str, namespace: &str) -> ResourceMetadata {
            ResourceMetadata {
                name: name.to_string(),
                namespace: namespace.to_string(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
            }
        }

        #[test]
        fn test_operator_creation() {
            let op = K8sOperator::new();
            assert_eq!(op.node_count(), 0);
            assert_eq!(op.lease_count(), 0);
            assert_eq!(op.steering_config_count(), 0);
        }

        #[test]
        fn test_add_node() {
            let mut op = K8sOperator::new();
            let node = NodeCRD {
                api_version: "ed2kia.io/v1".to_string(),
                kind: "Node".to_string(),
                metadata: make_metadata("node1", "default"),
                spec: NodeSpec {
                    replicas: 3,
                    image: "ed2kia/node:v2.0".to_string(),
                    cpu_limit: "1000m".to_string(),
                    memory_limit: "1Gi".to_string(),
                    port: 8080,
                },
                status: None,
            };
            op.add_node(node).unwrap();
            assert_eq!(op.node_count(), 1);
        }

        #[test]
        fn test_add_node_negative_replicas() {
            let mut op = K8sOperator::new();
            let node = NodeCRD {
                api_version: "ed2kia.io/v1".to_string(),
                kind: "Node".to_string(),
                metadata: make_metadata("bad", "default"),
                spec: NodeSpec {
                    replicas: -1,
                    image: "ed2kia/node:v2.0".to_string(),
                    cpu_limit: "1000m".to_string(),
                    memory_limit: "1Gi".to_string(),
                    port: 8080,
                },
                status: None,
            };
            assert_eq!(op.add_node(node), Err(K8sError::InvalidSchema("Replicas cannot be negative".to_string())));
        }

        #[test]
        fn test_add_node_empty_image() {
            let mut op = K8sOperator::new();
            let node = NodeCRD {
                api_version: "ed2kia.io/v1".to_string(),
                kind: "Node".to_string(),
                metadata: make_metadata("bad", "default"),
                spec: NodeSpec {
                    replicas: 1,
                    image: "".to_string(),
                    cpu_limit: "1000m".to_string(),
                    memory_limit: "1Gi".to_string(),
                    port: 8080,
                },
                status: None,
            };
            assert_eq!(op.add_node(node), Err(K8sError::InvalidSchema("Image cannot be empty".to_string())));
        }

        #[test]
        fn test_add_lease() {
            let mut op = K8sOperator::new();
            let lease = LeaseCRD {
                api_version: "ed2kia.io/v1".to_string(),
                kind: "Lease".to_string(),
                metadata: make_metadata("lease1", "default"),
                spec: LeaseSpec {
                    duration_seconds: 30,
                    renew_deadline_seconds: 15,
                    retry_period_seconds: 10,
                    holder_identity: "node1".to_string(),
                },
                status: None,
            };
            op.add_lease(lease).unwrap();
            assert_eq!(op.lease_count(), 1);
        }

        #[test]
        fn test_add_lease_invalid_duration() {
            let mut op = K8sOperator::new();
            let lease = LeaseCRD {
                api_version: "ed2kia.io/v1".to_string(),
                kind: "Lease".to_string(),
                metadata: make_metadata("bad", "default"),
                spec: LeaseSpec {
                    duration_seconds: 0,
                    renew_deadline_seconds: 15,
                    retry_period_seconds: 10,
                    holder_identity: "node1".to_string(),
                },
                status: None,
            };
            assert_eq!(op.add_lease(lease), Err(K8sError::InvalidSchema("Duration must be > 0".to_string())));
        }

        #[test]
        fn test_add_steering_config() {
            let mut op = K8sOperator::new();
            let config = SteeringConfigCRD {
                api_version: "ed2kia.io/v1".to_string(),
                kind: "SteeringConfig".to_string(),
                metadata: make_metadata("steer1", "default"),
                spec: SteeringConfigSpec {
                    empathy: 0.7,
                    creativity: 0.5,
                    safety: 0.9,
                    target_nodes: vec!["node1".to_string(), "node2".to_string()],
                },
                status: None,
            };
            op.add_steering_config(config).unwrap();
            assert_eq!(op.steering_config_count(), 1);
        }

        #[test]
        fn test_add_steering_config_invalid_empathy() {
            let mut op = K8sOperator::new();
            let config = SteeringConfigCRD {
                api_version: "ed2kia.io/v1".to_string(),
                kind: "SteeringConfig".to_string(),
                metadata: make_metadata("bad", "default"),
                spec: SteeringConfigSpec {
                    empathy: 1.5,
                    creativity: 0.5,
                    safety: 0.9,
                    target_nodes: vec![],
                },
                status: None,
            };
            assert_eq!(op.add_steering_config(config), Err(K8sError::InvalidSchema("Empathy must be 0.0-1.0".to_string())));
        }

        #[test]
        fn test_reconcile_empty() {
            let mut op = K8sOperator::new();
            let results = op.reconcile_all();
            assert_eq!(results.len(), 0);
        }

        #[test]
        fn test_reconcile_all() {
            let mut op = K8sOperator::new();

            op.add_node(NodeCRD {
                api_version: "ed2kia.io/v1".to_string(),
                kind: "Node".to_string(),
                metadata: make_metadata("node1", "default"),
                spec: NodeSpec {
                    replicas: 2,
                    image: "ed2kia/node:v2.0".to_string(),
                    cpu_limit: "1000m".to_string(),
                    memory_limit: "1Gi".to_string(),
                    port: 8080,
                },
                status: None,
            }).unwrap();

            op.add_lease(LeaseCRD {
                api_version: "ed2kia.io/v1".to_string(),
                kind: "Lease".to_string(),
                metadata: make_metadata("lease1", "default"),
                spec: LeaseSpec {
                    duration_seconds: 30,
                    renew_deadline_seconds: 15,
                    retry_period_seconds: 10,
                    holder_identity: "node1".to_string(),
                },
                status: None,
            }).unwrap();

            let results = op.reconcile_all();
            assert_eq!(results.len(), 2);
            assert!(results.iter().all(|r| r.success));
        }

        #[test]
        fn test_json_serialization() {
            let node = NodeCRD {
                api_version: "ed2kia.io/v1".to_string(),
                kind: "Node".to_string(),
                metadata: make_metadata("node1", "default"),
                spec: NodeSpec {
                    replicas: 3,
                    image: "ed2kia/node:v2.0".to_string(),
                    cpu_limit: "1000m".to_string(),
                    memory_limit: "1Gi".to_string(),
                    port: 8080,
                },
                status: Some(NodeStatus {
                    ready: true,
                    current_replicas: 3,
                    desired_replicas: 3,
                    conditions: vec!["Ready".to_string()],
                }),
            };

            let json = serde_json::to_string(&node).unwrap();
            assert!(json.contains("ed2kia.io/v1"));
            assert!(json.contains("node1"));

            let deserialized: NodeCRD = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.metadata.name, "node1");
            assert_eq!(deserialized.spec.replicas, 3);
        }

        #[test]
        fn test_error_display() {
            let err = K8sError::NotFound("resource".to_string());
            assert!(err.to_string().contains("resource"));
        }

        #[test]
        fn test_operator_default() {
            let op = K8sOperator::default();
            assert_eq!(op.node_count(), 0);
        }

        #[test]
        fn test_full_lifecycle() {
            let mut op = K8sOperator::new();

            // Add resources
            op.add_node(NodeCRD {
                api_version: "ed2kia.io/v1".to_string(),
                kind: "Node".to_string(),
                metadata: make_metadata("node1", "production"),
                spec: NodeSpec {
                    replicas: 5,
                    image: "ed2kia/node:v2.0".to_string(),
                    cpu_limit: "2000m".to_string(),
                    memory_limit: "4Gi".to_string(),
                    port: 8080,
                },
                status: None,
            }).unwrap();

            op.add_steering_config(SteeringConfigCRD {
                api_version: "ed2kia.io/v1".to_string(),
                kind: "SteeringConfig".to_string(),
                metadata: make_metadata("steer1", "production"),
                spec: SteeringConfigSpec {
                    empathy: 0.8,
                    creativity: 0.6,
                    safety: 0.95,
                    target_nodes: vec!["node1".to_string()],
                },
                status: None,
            }).unwrap();

            // Reconcile
            let results = op.reconcile_all();
            assert_eq!(results.len(), 2);
            assert!(results.iter().all(|r| r.success));

            // Verify log
            assert_eq!(op.reconcile_log().len(), 2);
        }
    }
}

pub use internal::{
    K8sError, K8sOperator, LeaseCRD, LeaseSpec, LeaseStatus, NodeCRD, NodeSpec, NodeStatus,
    ReconcileResult, ResourceMetadata, SteeringConfigCRD, SteeringConfigSpec, SteeringConfigStatus,
};
