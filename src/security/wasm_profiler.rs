//! WASM Execution Profiler - Dynamic resource tracking per execution session
//!
//! This module provides fine-grained profiling of WASM module executions,
//! tracking memory usage, CPU consumption (via wasmtime fuel), wall-clock time,
//! and instruction counts. It enforces resource thresholds and triggers alerts
//! when limits are approached or exceeded.
//!
//! # Architecture
//!
//! - [`Profiler`] manages profiling sessions and aggregates statistics
//! - [`ExecutionProfile`] holds per-session resource measurements
//! - [`ProfilingAlert`] signals when thresholds are exceeded
//! - [`ProfilerStats`] provides aggregate statistics across all sessions
//!
//! # Threshold Alerts
//!
//! | Alert | Condition |
//! |-------|-----------|
//! | `MemoryHigh` | Memory usage > 80% of limit |
//! | `MemoryCritical` | Memory usage > 95% of limit |
//! | `FuelExhausted` | Fuel limit reached |
//! | `TimeoutExceeded` | Execution time exceeded |
//!
//! # License
//!
//! Apache 2.0 + Ethical Use Clause

#[cfg(feature = "v1.1-sprint1")]
use tracing::debug;

/// Unique identifier for a profiling session.
#[cfg(feature = "v1.1-sprint1")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

/// Execution profile capturing resource usage for a single WASM execution session.
#[cfg(feature = "v1.1-sprint1")]
#[derive(Debug, Clone)]
pub struct ExecutionProfile {
    /// Peak memory usage in bytes during the session.
    pub memory_bytes_peak: usize,
    /// Current memory usage in bytes at the time of measurement.
    pub memory_bytes_current: usize,
    /// CPU cycles consumed (approximated via fuel counter).
    pub cpu_cycles: u64,
    /// Wall-clock time in milliseconds.
    pub wall_time_ms: f64,
    /// Number of instructions executed (approximated).
    pub instructions_executed: u64,
    /// Fuel consumed by wasmtime's fuel counter.
    pub fuel_consumed: u64,
}

impl ExecutionProfile {
    /// Create a new empty execution profile.
    pub fn new() -> Self {
        Self {
            memory_bytes_peak: 0,
            memory_bytes_current: 0,
            cpu_cycles: 0,
            wall_time_ms: 0.0,
            instructions_executed: 0,
            fuel_consumed: 0,
        }
    }

    /// Calculate memory usage as a percentage of the given limit.
    pub fn memory_usage_percent(&self, limit_bytes: usize) -> f64 {
        if limit_bytes == 0 {
            return 100.0;
        }
        (self.memory_bytes_peak as f64 / limit_bytes as f64) * 100.0
    }
}

impl Default for ExecutionProfile {
    fn default() -> Self {
        Self::new()
    }
}

/// Alert indicating resource threshold violations.
#[cfg(feature = "v1.1-sprint1")]
#[derive(Debug, Clone, PartialEq)]
pub enum ProfilingAlert {
    /// All resource usage within acceptable limits.
    Ok,
    /// Memory usage exceeds 80% of the configured limit. Contains usage percentage.
    MemoryHigh(f64),
    /// Memory usage exceeds 95% of the configured limit. Contains usage percentage.
    MemoryCritical(f64),
    /// Fuel limit has been exhausted.
    FuelExhausted,
    /// Execution time exceeded the allowed timeout. Contains elapsed milliseconds.
    TimeoutExceeded(f64),
}

/// Aggregate statistics across all profiling sessions.
#[cfg(feature = "v1.1-sprint1")]
#[derive(Debug, Clone)]
pub struct ProfilerStats {
    /// Total number of profiling sessions completed.
    pub total_sessions: usize,
    /// Average peak memory usage in bytes across all sessions.
    pub avg_memory_bytes: f64,
    /// Average fuel consumed across all sessions.
    pub avg_fuel_consumed: f64,
    /// Average wall-clock time in milliseconds across all sessions.
    pub avg_wall_time_ms: f64,
    /// Total number of alerts triggered across all sessions.
    pub alerts_triggered: usize,
}

impl ProfilerStats {
    /// Create a new empty profiler stats.
    pub fn new() -> Self {
        Self {
            total_sessions: 0,
            avg_memory_bytes: 0.0,
            avg_fuel_consumed: 0.0,
            avg_wall_time_ms: 0.0,
            alerts_triggered: 0,
        }
    }
}

impl Default for ProfilerStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Profiler for tracking WASM execution resources.
///
/// Manages multiple profiling sessions concurrently, recording memory, fuel,
/// and timing data. Provides threshold checking and aggregate statistics.
#[cfg(feature = "v1.1-sprint1")]
pub struct Profiler {
    /// Peak memory limit in bytes for threshold alerts.
    memory_limit_bytes: usize,
    /// Fuel limit for threshold alerts.
    fuel_limit: u64,
    /// Active profiling sessions.
    sessions: std::collections::HashMap<String, ExecutionProfile>,
    /// Completed session profiles.
    completed_profiles: std::collections::HashMap<String, ExecutionProfile>,
    /// Total alerts triggered.
    alerts_triggered: usize,
}

impl Profiler {
    /// Create a new profiler with the given resource limits.
    ///
    /// # Arguments
    ///
    /// * `memory_limit_bytes` - Peak memory limit in bytes for threshold alerts.
    /// * `fuel_limit` - Fuel limit for threshold alerts.
    pub fn new(memory_limit_bytes: usize, fuel_limit: u64) -> Self {
        debug!(
            "Profiler created: memory_limit={} bytes, fuel_limit={}",
            memory_limit_bytes, fuel_limit
        );
        Self {
            memory_limit_bytes,
            fuel_limit,
            sessions: std::collections::HashMap::new(),
            completed_profiles: std::collections::HashMap::new(),
            alerts_triggered: 0,
        }
    }

    /// Begin a new profiling session and return its unique identifier.
    ///
    /// The session starts with zeroed resource measurements.
    pub fn start_session(&mut self) -> SessionId {
        let id = uuid::Uuid::new_v4().to_string();
        let profile = ExecutionProfile::new();
        self.sessions.insert(id.clone(), profile);
        debug!("Profiling session started: {}", id);
        SessionId(id)
    }

    /// Record a memory snapshot for the given session.
    ///
    /// Updates both peak and current memory usage. Peak memory is only
    /// increased when a higher value is recorded.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The identifier of the profiling session.
    /// * `bytes` - Current memory usage in bytes.
    pub fn record_memory(&mut self, session_id: &SessionId, bytes: usize) {
        if let Some(profile) = self.sessions.get_mut(&session_id.0) {
            profile.memory_bytes_current = bytes;
            if bytes > profile.memory_bytes_peak {
                profile.memory_bytes_peak = bytes;
            }
            debug!(
                "Session {}: memory recorded peak={} current={}",
                session_id.0, profile.memory_bytes_peak, bytes
            );
        }
    }

    /// Record fuel consumption for the given session.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The identifier of the profiling session.
    /// * `fuel` - Fuel consumed so far.
    pub fn record_fuel(&mut self, session_id: &SessionId, fuel: u64) {
        if let Some(profile) = self.sessions.get_mut(&session_id.0) {
            profile.fuel_consumed = fuel;
            // Approximate instructions from fuel (1:1 mapping for wasmtime fuel)
            profile.instructions_executed = fuel;
            // Approximate CPU cycles (rough estimate: 1 fuel ~ 10 cycles)
            profile.cpu_cycles = fuel.saturating_mul(10);
            debug!("Session {}: fuel recorded={}", session_id.0, fuel);
        }
    }

    /// Record wall-clock time for the given session.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The identifier of the profiling session.
    /// * `elapsed_ms` - Elapsed time in milliseconds.
    pub fn record_time(&mut self, session_id: &SessionId, elapsed_ms: f64) {
        if let Some(profile) = self.sessions.get_mut(&session_id.0) {
            profile.wall_time_ms = elapsed_ms;
            debug!("Session {}: wall_time={}ms", session_id.0, elapsed_ms);
        }
    }

    /// Finalize a session and make its profile available for retrieval.
    ///
    /// Moves the session from active to completed profiles.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The identifier of the profiling session to finalize.
    pub fn finalize_session(&mut self, session_id: &SessionId) -> Option<ExecutionProfile> {
        let profile = self.sessions.remove(&session_id.0)?;
        self.completed_profiles
            .insert(session_id.0.clone(), profile.clone());
        debug!("Session {} finalized", session_id.0);
        Some(profile)
    }

    /// Get the completed profile for a session.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The identifier of the profiling session.
    pub fn get_profile(&self, session_id: &SessionId) -> Option<ExecutionProfile> {
        self.completed_profiles.get(&session_id.0).cloned()
    }

    /// Check resource thresholds and return an alert if limits are exceeded.
    ///
    /// # Arguments
    ///
    /// * `profile` - The execution profile to check.
    ///
    /// # Returns
    ///
    /// A [`ProfilingAlert`] indicating the threshold status.
    pub fn check_thresholds(&mut self, profile: &ExecutionProfile) -> ProfilingAlert {
        let memory_percent = profile.memory_usage_percent(self.memory_limit_bytes);

        // Check memory thresholds first (most critical)
        if memory_percent > 95.0 {
            self.alerts_triggered += 1;
            return ProfilingAlert::MemoryCritical(memory_percent);
        }
        if memory_percent > 80.0 {
            self.alerts_triggered += 1;
            return ProfilingAlert::MemoryHigh(memory_percent);
        }

        // Check fuel exhaustion
        if profile.fuel_consumed >= self.fuel_limit {
            self.alerts_triggered += 1;
            return ProfilingAlert::FuelExhausted;
        }

        // Check timeout (default: 30 seconds)
        let timeout_ms = 30_000.0;
        if profile.wall_time_ms > timeout_ms {
            self.alerts_triggered += 1;
            return ProfilingAlert::TimeoutExceeded(profile.wall_time_ms);
        }

        ProfilingAlert::Ok
    }

    /// Get aggregate statistics across all completed sessions.
    pub fn get_stats(&self) -> ProfilerStats {
        let completed = &self.completed_profiles;
        let total = completed.len();

        if total == 0 {
            return ProfilerStats {
                alerts_triggered: self.alerts_triggered,
                ..Default::default()
            };
        }

        let total_memory: f64 = completed.values().map(|p| p.memory_bytes_peak as f64).sum();
        let total_fuel: f64 = completed.values().map(|p| p.fuel_consumed as f64).sum();
        let total_time: f64 = completed.values().map(|p| p.wall_time_ms).sum();

        ProfilerStats {
            total_sessions: total,
            avg_memory_bytes: total_memory / total as f64,
            avg_fuel_consumed: total_fuel / total as f64,
            avg_wall_time_ms: total_time / total as f64,
            alerts_triggered: self.alerts_triggered,
        }
    }

    /// Get the configured memory limit in bytes.
    pub fn memory_limit(&self) -> usize {
        self.memory_limit_bytes
    }

    /// Get the configured fuel limit.
    pub fn fuel_limit(&self) -> u64 {
        self.fuel_limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_creation() {
        let profiler = Profiler::new(256 * 1024 * 1024, 100_000_000);
        assert_eq!(profiler.memory_limit(), 256 * 1024 * 1024);
        assert_eq!(profiler.fuel_limit(), 100_000_000);
    }

    #[test]
    fn test_session_lifecycle() {
        let mut profiler = Profiler::new(256 * 1024 * 1024, 100_000_000);

        let session = profiler.start_session();
        profiler.record_memory(&session, 1024);
        profiler.record_fuel(&session, 500);
        profiler.record_time(&session, 10.5);

        let profile = profiler
            .finalize_session(&session)
            .expect("session should exist");
        assert_eq!(profile.memory_bytes_peak, 1024);
        assert_eq!(profile.memory_bytes_current, 1024);
        assert_eq!(profile.fuel_consumed, 500);
        assert_eq!(profile.wall_time_ms, 10.5);

        let retrieved = profiler
            .get_profile(&session)
            .expect("profile should exist");
        assert_eq!(retrieved.memory_bytes_peak, 1024);
    }

    #[test]
    fn test_memory_peak_tracking() {
        let mut profiler = Profiler::new(256 * 1024 * 1024, 100_000_000);

        let session = profiler.start_session();
        profiler.record_memory(&session, 512);
        profiler.record_memory(&session, 2048);
        profiler.record_memory(&session, 1024);

        let profile = profiler
            .finalize_session(&session)
            .expect("session should exist");
        assert_eq!(profile.memory_bytes_peak, 2048);
        assert_eq!(profile.memory_bytes_current, 1024);
    }

    #[test]
    fn test_threshold_ok() {
        let mut profiler = Profiler::new(1_000_000, 100_000);

        let profile = ExecutionProfile {
            memory_bytes_peak: 500_000,
            memory_bytes_current: 500_000,
            cpu_cycles: 0,
            wall_time_ms: 10.0,
            instructions_executed: 0,
            fuel_consumed: 50_000,
        };

        assert_eq!(profiler.check_thresholds(&profile), ProfilingAlert::Ok);
    }

    #[test]
    fn test_threshold_memory_high() {
        let mut profiler = Profiler::new(1_000_000, 100_000);

        let profile = ExecutionProfile {
            memory_bytes_peak: 850_000,
            memory_bytes_current: 850_000,
            cpu_cycles: 0,
            wall_time_ms: 10.0,
            instructions_executed: 0,
            fuel_consumed: 50_000,
        };

        let alert = profiler.check_thresholds(&profile);
        match alert {
            ProfilingAlert::MemoryHigh(percent) => {
                assert!(percent > 80.0 && percent <= 95.0);
            }
            _ => panic!("Expected MemoryHigh alert, got {:?}", alert),
        }
    }

    #[test]
    fn test_threshold_memory_critical() {
        let mut profiler = Profiler::new(1_000_000, 100_000);

        let profile = ExecutionProfile {
            memory_bytes_peak: 960_000,
            memory_bytes_current: 960_000,
            cpu_cycles: 0,
            wall_time_ms: 10.0,
            instructions_executed: 0,
            fuel_consumed: 50_000,
        };

        let alert = profiler.check_thresholds(&profile);
        match alert {
            ProfilingAlert::MemoryCritical(percent) => {
                assert!(percent > 95.0);
            }
            _ => panic!("Expected MemoryCritical alert, got {:?}", alert),
        }
    }

    #[test]
    fn test_threshold_fuel_exhausted() {
        let mut profiler = Profiler::new(1_000_000, 100_000);

        let profile = ExecutionProfile {
            memory_bytes_peak: 100_000,
            memory_bytes_current: 100_000,
            cpu_cycles: 0,
            wall_time_ms: 10.0,
            instructions_executed: 100_000,
            fuel_consumed: 100_000,
        };

        assert_eq!(
            profiler.check_thresholds(&profile),
            ProfilingAlert::FuelExhausted
        );
    }

    #[test]
    fn test_threshold_timeout() {
        let mut profiler = Profiler::new(1_000_000, 100_000);

        let profile = ExecutionProfile {
            memory_bytes_peak: 100_000,
            memory_bytes_current: 100_000,
            cpu_cycles: 0,
            wall_time_ms: 35_000.0,
            instructions_executed: 0,
            fuel_consumed: 50_000,
        };

        let alert = profiler.check_thresholds(&profile);
        match alert {
            ProfilingAlert::TimeoutExceeded(ms) => {
                assert!(ms > 30_000.0);
            }
            _ => panic!("Expected TimeoutExceeded alert, got {:?}", alert),
        }
    }

    #[test]
    fn test_stats_aggregation() {
        let mut profiler = Profiler::new(1_000_000, 100_000);

        // Create and finalize 3 sessions
        for i in 1..=3 {
            let session = profiler.start_session();
            profiler.record_memory(&session, i * 1000);
            profiler.record_fuel(&session, (i * 500) as u64);
            profiler.record_time(&session, i as f64 * 10.0);
            profiler.finalize_session(&session);
        }

        let stats = profiler.get_stats();
        assert_eq!(stats.total_sessions, 3);
        // Avg memory: (1000 + 2000 + 3000) / 3 = 2000
        assert!((stats.avg_memory_bytes - 2000.0).abs() < 1.0);
        // Avg fuel: (500 + 1000 + 1500) / 3 = 1000
        assert!((stats.avg_fuel_consumed - 1000.0).abs() < 1.0);
        // Avg time: (10 + 20 + 30) / 3 = 20
        assert!((stats.avg_wall_time_ms - 20.0).abs() < 1.0);
    }

    #[test]
    fn test_empty_stats() {
        let profiler = Profiler::new(1_000_000, 100_000);
        let stats = profiler.get_stats();
        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.avg_memory_bytes, 0.0);
    }

    #[test]
    fn test_memory_usage_percent() {
        let profile = ExecutionProfile {
            memory_bytes_peak: 800_000,
            memory_bytes_current: 800_000,
            cpu_cycles: 0,
            wall_time_ms: 0.0,
            instructions_executed: 0,
            fuel_consumed: 0,
        };

        let percent = profile.memory_usage_percent(1_000_000);
        assert!((percent - 80.0).abs() < 0.01);
    }

    #[test]
    fn test_memory_usage_percent_zero_limit() {
        let profile = ExecutionProfile::new();
        let percent = profile.memory_usage_percent(0);
        assert_eq!(percent, 100.0);
    }

    #[test]
    fn test_fuel_approximates_instructions() {
        let mut profiler = Profiler::new(1_000_000, 100_000);

        let session = profiler.start_session();
        profiler.record_fuel(&session, 1000);

        let profile = profiler.sessions.get(&session.0).expect("session exists");
        assert_eq!(profile.instructions_executed, 1000);
        assert_eq!(profile.cpu_cycles, 10_000);
    }
}
