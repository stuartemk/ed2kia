//! Schema Negotiator — Binary schema negotiation for cross-protocol compatibility.
//!
//! Provides schema version negotiation, field mapping, and compatibility checking
//! between different protocol implementations. Ensures safe data exchange without
//! dynamic reflection.
//!
//! **Design:** Version-based schema negotiation with field compatibility matrix.
//! Zero financial logic — operates on compute credits and technical state only.
//!
//! Apache License 2.0 + Ethical Use Clause

#[cfg(feature = "v1.6-sprint1")]
mod internal {
    use std::collections::HashMap;

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    /// Errors for schema negotiation.
    #[derive(Debug, Clone, PartialEq)]
    pub enum SchemaError {
        /// Schema version not supported.
        VersionNotSupported(u32),
        /// Field incompatible.
        FieldIncompatible(String),
        /// Negotiation failed.
        NegotiationFailed(String),
        /// Schema not found.
        SchemaNotFound(String),
    }

    impl std::fmt::Display for SchemaError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                SchemaError::VersionNotSupported(v) => {
                    write!(f, "Schema version {} not supported", v)
                }
                SchemaError::FieldIncompatible(field) => write!(f, "Field {} incompatible", field),
                SchemaError::NegotiationFailed(msg) => write!(f, "Negotiation failed: {}", msg),
                SchemaError::SchemaNotFound(name) => write!(f, "Schema {} not found", name),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Schema Definition
    // ---------------------------------------------------------------------------

    /// Schema field definition.
    #[derive(Debug, Clone)]
    pub struct SchemaField {
        pub name: String,
        pub field_type: String,
        pub required: bool,
    }

    /// Schema definition with version and fields.
    #[derive(Debug, Clone)]
    pub struct SchemaDefinition {
        pub name: String,
        pub version: u32,
        pub fields: Vec<SchemaField>,
    }

    impl SchemaDefinition {
        pub fn new(name: String, version: u32) -> Self {
            Self {
                name,
                version,
                fields: Vec::new(),
            }
        }

        pub fn add_field(&mut self, name: String, field_type: String, required: bool) {
            self.fields.push(SchemaField {
                name,
                field_type,
                required,
            });
        }
    }

    // ---------------------------------------------------------------------------
    // Negotiation Result
    // ---------------------------------------------------------------------------

    /// Result of schema negotiation.
    #[derive(Debug, Clone)]
    pub struct NegotiationResult {
        pub compatible: bool,
        pub common_version: u32,
        pub common_fields: Vec<String>,
        pub missing_fields: Vec<String>,
        pub extra_fields: Vec<String>,
    }

    // ---------------------------------------------------------------------------
    // Negotiation Stats
    // ---------------------------------------------------------------------------

    /// Statistics for schema negotiations.
    #[derive(Debug, Clone)]
    pub struct NegotiationStats {
        pub negotiations: u64,
        pub successful: u64,
        pub failed: u64,
        pub avg_time_ms: f64,
    }

    impl Default for NegotiationStats {
        fn default() -> Self {
            Self {
                negotiations: 0,
                successful: 0,
                failed: 0,
                avg_time_ms: 0.0,
            }
        }
    }

    impl NegotiationStats {
        pub fn record_success(&mut self, time_ms: u64) {
            self.negotiations += 1;
            self.successful += 1;
            self.avg_time_ms = self.avg_time_ms * 0.9 + time_ms as f64 * 0.1;
        }

        pub fn record_failure(&mut self) {
            self.negotiations += 1;
            self.failed += 1;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // ---------------------------------------------------------------------------
    // Schema Negotiator
    // ---------------------------------------------------------------------------

    /// Schema negotiator for cross-protocol compatibility.
    #[derive(Debug, Clone)]
    pub struct SchemaNegotiator {
        schemas: HashMap<String, Vec<SchemaDefinition>>,
        stats: NegotiationStats,
    }

    impl SchemaNegotiator {
        /// Create a new schema negotiator.
        pub fn new() -> Self {
            Self {
                schemas: HashMap::new(),
                stats: NegotiationStats::default(),
            }
        }

        /// Register a schema definition.
        pub fn register_schema(&mut self, schema: SchemaDefinition) {
            self.schemas
                .entry(schema.name.clone())
                .or_default()
                .push(schema);
        }

        /// Negotiate compatibility between two schemas.
        pub fn negotiate(
            &mut self,
            schema_a_name: &str,
            version_a: u32,
            schema_b_name: &str,
            version_b: u32,
        ) -> Result<NegotiationResult, SchemaError> {
            let start = std::time::Instant::now();

            let schema_a = self
                .schemas
                .get(schema_a_name)
                .ok_or_else(|| SchemaError::SchemaNotFound(schema_a_name.to_string()))?
                .iter()
                .find(|s| s.version == version_a)
                .ok_or(SchemaError::VersionNotSupported(version_a))?;

            let schema_b = self
                .schemas
                .get(schema_b_name)
                .ok_or_else(|| SchemaError::SchemaNotFound(schema_b_name.to_string()))?
                .iter()
                .find(|s| s.version == version_b)
                .ok_or(SchemaError::VersionNotSupported(version_b))?;

            // Find common fields
            let fields_a: Vec<&str> = schema_a.fields.iter().map(|f| f.name.as_str()).collect();
            let fields_b: Vec<&str> = schema_b.fields.iter().map(|f| f.name.as_str()).collect();

            let common_fields: Vec<String> = fields_a
                .iter()
                .filter(|f| fields_b.contains(f))
                .map(|s| s.to_string())
                .collect();

            let missing_fields: Vec<String> = fields_a
                .iter()
                .filter(|f| !fields_b.contains(f))
                .map(|s| s.to_string())
                .collect();

            let extra_fields: Vec<String> = fields_b
                .iter()
                .filter(|f| !fields_a.contains(f))
                .map(|s| s.to_string())
                .collect();

            // Check required fields compatibility
            let required_a: Vec<&str> = schema_a
                .fields
                .iter()
                .filter(|f| f.required)
                .map(|f| f.name.as_str())
                .collect();

            let compatible = required_a.iter().all(|f| fields_b.contains(f));

            // Common version is the lower of the two
            let common_version = version_a.min(version_b);

            let time_ms = start.elapsed().as_millis() as u64;

            let result = NegotiationResult {
                compatible,
                common_version,
                common_fields,
                missing_fields,
                extra_fields,
            };

            if compatible {
                self.stats.record_success(time_ms);
            } else {
                self.stats.record_failure();
            }

            Ok(result)
        }

        /// Get the latest version of a schema.
        pub fn get_latest_version(&self, name: &str) -> Option<u32> {
            self.schemas
                .get(name)
                .map(|versions| versions.iter().map(|s| s.version).max().unwrap_or(0))
        }

        /// Get negotiation statistics.
        pub fn get_stats(&self) -> &NegotiationStats {
            &self.stats
        }

        /// Reset statistics.
        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }
    }

    impl Default for SchemaNegotiator {
        fn default() -> Self {
            Self::new()
        }
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        fn make_schema(name: &str, version: u32) -> SchemaDefinition {
            let mut schema = SchemaDefinition::new(name.to_string(), version);
            schema.add_field("id".to_string(), "uint64".to_string(), true);
            schema.add_field("data".to_string(), "bytes".to_string(), true);
            schema.add_field("metadata".to_string(), "string".to_string(), false);
            schema
        }

        #[test]
        fn test_negotiator_creation() {
            let negotiator = SchemaNegotiator::default();
            assert_eq!(negotiator.schemas.len(), 0);
        }

        #[test]
        fn test_register_schema() {
            let mut negotiator = SchemaNegotiator::default();
            let schema = make_schema("test", 1);
            negotiator.register_schema(schema);
            assert_eq!(negotiator.schemas.len(), 1);
        }

        #[test]
        fn test_negotiate_compatible() {
            let mut negotiator = SchemaNegotiator::default();
            negotiator.register_schema(make_schema("proto_a", 1));
            negotiator.register_schema(make_schema("proto_b", 1));

            let result = negotiator.negotiate("proto_a", 1, "proto_b", 1).unwrap();
            assert!(result.compatible);
            assert_eq!(result.common_version, 1);
        }

        #[test]
        fn test_negotiate_incompatible() {
            let mut negotiator = SchemaNegotiator::default();
            negotiator.register_schema(make_schema("proto_a", 1));

            let mut schema_b = SchemaDefinition::new("proto_b".to_string(), 1);
            schema_b.add_field("other".to_string(), "string".to_string(), false);
            negotiator.register_schema(schema_b);

            let result = negotiator.negotiate("proto_a", 1, "proto_b", 1).unwrap();
            assert!(!result.compatible);
        }

        #[test]
        fn test_schema_not_found() {
            let mut negotiator = SchemaNegotiator::default();
            match negotiator
                .negotiate("unknown", 1, "also_unknown", 1)
                .unwrap_err()
            {
                SchemaError::SchemaNotFound(name) => assert_eq!(name, "unknown"),
                e => panic!("Expected SchemaNotFound, got {:?}", e),
            }
        }

        #[test]
        fn test_version_not_supported() {
            let mut negotiator = SchemaNegotiator::default();
            negotiator.register_schema(make_schema("test", 1));
            match negotiator.negotiate("test", 99, "test", 1).unwrap_err() {
                SchemaError::VersionNotSupported(v) => assert_eq!(v, 99),
                e => panic!("Expected VersionNotSupported, got {:?}", e),
            }
        }

        #[test]
        fn test_get_latest_version() {
            let mut negotiator = SchemaNegotiator::default();
            negotiator.register_schema(make_schema("test", 1));
            negotiator.register_schema(make_schema("test", 2));
            negotiator.register_schema(make_schema("test", 3));
            assert_eq!(negotiator.get_latest_version("test"), Some(3));
        }

        #[test]
        fn test_get_latest_version_missing() {
            let negotiator = SchemaNegotiator::default();
            assert_eq!(negotiator.get_latest_version("unknown"), None);
        }

        #[test]
        fn test_stats_recording() {
            let mut negotiator = SchemaNegotiator::default();
            negotiator.register_schema(make_schema("a", 1));
            negotiator.register_schema(make_schema("b", 1));
            negotiator.negotiate("a", 1, "b", 1).unwrap();
            let stats = negotiator.get_stats();
            assert_eq!(stats.negotiations, 1);
            assert_eq!(stats.successful, 1);
        }

        #[test]
        fn test_reset_stats() {
            let mut negotiator = SchemaNegotiator::default();
            negotiator.register_schema(make_schema("a", 1));
            negotiator.register_schema(make_schema("b", 1));
            negotiator.negotiate("a", 1, "b", 1).unwrap();
            negotiator.reset_stats();
            assert_eq!(negotiator.get_stats().negotiations, 0);
        }

        #[test]
        fn test_error_display() {
            let err = SchemaError::SchemaNotFound("test".to_string());
            assert!(!format!("{}", err).is_empty());
        }
    }
}

#[cfg(feature = "v1.6-sprint1")]
pub use internal::*;
