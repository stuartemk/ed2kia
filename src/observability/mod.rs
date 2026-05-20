//! Observability — Prometheus/Grafana metrics collection (v2.1.0-sprint11)
//!
//! Production-ready Prometheus metrics for network health, consensus, reputation,
//! RLHF feedback and WASM worker observability. Zero telemetry, zero external calls.
//! Metrics are strictly for network health and alignment monitoring.
//!
//! **Feature gate:** `v2.1-observability`
//! **License:** Apache 2.0 + Ethical Use Clause

/// Prometheus metrics registry and collectors
#[cfg(feature = "v2.1-observability")]
pub mod metrics;

/// Health check endpoint
#[cfg(feature = "v2.1-observability")]
pub mod health_check;
