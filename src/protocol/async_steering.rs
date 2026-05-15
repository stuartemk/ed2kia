//! Async Steering v1 — Late correction signals for distributed tensor pipelines (RFC-001 §2.4).
//!
//! Provides async channel-based steering signals that allow downstream consumers
//! to apply late corrections to context windows when upstream data arrives delayed.
//!
//! v1.8 additions: backpressure handling, latency metrics, timeout tracking.

mod internal {
    use std::collections::HashMap;
    use std::fmt;
    use std::time::Instant;

    // ============================================================================
    // Errors
    // ============================================================================

    /// Async steering errors
    #[derive(Debug, Clone, PartialEq)]
    pub enum AsyncSteeringError {
        /// Channel closed
        ChannelClosed,
        /// Context window too small for correction
        WindowTooSmall,
        /// Signal magnitude exceeds bounds
        SignalOutOfBounds,
        /// Channel full — backpressure active
        Backpressure,
        /// Signal timed out before processing
        Timeout,
    }

    impl fmt::Display for AsyncSteeringError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                AsyncSteeringError::ChannelClosed => write!(f, "async_steering: channel closed"),
                AsyncSteeringError::WindowTooSmall => {
                    write!(f, "async_steering: context window too small for correction")
                }
                AsyncSteeringError::SignalOutOfBounds => {
                    write!(f, "async_steering: signal magnitude exceeds [-1.0, 1.0] bounds")
                }
                AsyncSteeringError::Backpressure => {
                    write!(f, "async_steering: backpressure")
                }
                AsyncSteeringError::Timeout => {
                    write!(f, "async_steering: timeout")
                }
            }
        }
    }

    impl std::error::Error for AsyncSteeringError {}

    // ============================================================================
    // Steering Signal
    // ============================================================================

    /// A late correction signal with metadata.
    #[derive(Debug, Clone, PartialEq)]
    pub struct SteeringSignal {
        /// Signal value in [-1.0, 1.0]
        pub value: f32,
        /// Delay in milliseconds since original context was created
        pub delay_ms: u64,
        /// Source identifier (e.g., federation_id, node_id)
        pub source: String,
        /// Sequence number for ordering
        pub seq: u64,
    }

    impl SteeringSignal {
        /// Create a new steering signal.
        ///
        /// # Arguments
        /// * `value` - Correction value in [-1.0, 1.0]
        /// * `delay_ms` - Delay since original context
        /// * `source` - Source identifier
        /// * `seq` - Sequence number
        ///
        /// # Errors
        /// * `AsyncSteeringError::SignalOutOfBounds` if value outside [-1.0, 1.0]
        pub fn new(value: f32, delay_ms: u64, source: String, seq: u64) -> Result<Self, AsyncSteeringError> {
            if value < -1.0 || value > 1.0 {
                return Err(AsyncSteeringError::SignalOutOfBounds);
            }
            Ok(Self {
                value,
                delay_ms,
                source,
                seq,
            })
        }
    }

    // ============================================================================
    // Async Steering Channel
    // ============================================================================

    /// Synchronous steering channel for testing and baseline PoC.
    ///
    /// Uses a simple VecDeque buffer. In production, replace with tokio::sync::mpsc.
    pub struct AsyncSteeringChannelMock {
        buffer: std::collections::VecDeque<SteeringSignal>,
        capacity: usize,
    }

    impl AsyncSteeringChannelMock {
        /// Create a new mock channel with bounded capacity.
        pub fn new(capacity: usize) -> Self {
            Self {
                buffer: std::collections::VecDeque::with_capacity(capacity),
                capacity,
            }
        }

        /// Send a steering signal (sync).
        pub fn send(&mut self, signal: SteeringSignal) -> Result<(), AsyncSteeringError> {
            if self.buffer.len() >= self.capacity {
                return Err(AsyncSteeringError::ChannelClosed);
            }
            self.buffer.push_back(signal);
            Ok(())
        }

        /// Try to send without blocking. Returns Backpressure when full.
        pub fn try_send(&mut self, signal: SteeringSignal) -> Result<(), AsyncSteeringError> {
            if self.buffer.len() >= self.capacity {
                return Err(AsyncSteeringError::Backpressure);
            }
            self.buffer.push_back(signal);
            Ok(())
        }

        /// Receive the next steering signal (sync).
        pub fn recv(&mut self) -> Option<SteeringSignal> {
            self.buffer.pop_front()
        }

        /// Get channel capacity.
        pub fn capacity(&self) -> usize {
            self.capacity
        }

        /// Get current buffer length.
        pub fn len(&self) -> usize {
            self.buffer.len()
        }

        /// Check if buffer is empty.
        pub fn is_empty(&self) -> bool {
            self.buffer.is_empty()
        }
    }

    // ============================================================================
    // Steering Metrics
    // ============================================================================

    /// Metrics for async steering channel performance.
    #[derive(Debug, Clone, Default)]
    pub struct SteeringMetrics {
        /// Total signals sent successfully
        pub total_sent: u64,
        /// Total signals received
        pub total_received: u64,
        /// Total signals dropped due to backpressure
        pub total_dropped: u64,
        /// Total timeouts
        pub total_timeouts: u64,
        /// Cumulative latency in milliseconds
        pub cumulative_latency_ms: f64,
    }

    impl SteeringMetrics {
        /// Record a successful send with latency.
        pub fn record_send(&mut self, latency_ms: f64) {
            self.total_sent += 1;
            self.cumulative_latency_ms += latency_ms;
        }

        /// Record a successful receive.
        pub fn record_recv(&mut self) {
            self.total_received += 1;
        }

        /// Record a dropped signal (backpressure).
        pub fn record_drop(&mut self) {
            self.total_dropped += 1;
        }

        /// Record a timeout.
        pub fn record_timeout(&mut self) {
            self.total_timeouts += 1;
        }

        /// Calculate average latency in milliseconds.
        pub fn avg_latency_ms(&self) -> f64 {
            if self.total_sent == 0 {
                return 0.0;
            }
            self.cumulative_latency_ms / (self.total_sent as f64)
        }

        /// Calculate drop rate as fraction of total attempts.
        pub fn drop_rate(&self) -> f64 {
            let total = self.total_sent + self.total_dropped;
            if total == 0 {
                return 0.0;
            }
            (self.total_dropped as f64) / (total as f64)
        }

        /// Reset all metrics to zero.
        pub fn reset(&mut self) {
            self.total_sent = 0;
            self.total_received = 0;
            self.total_dropped = 0;
            self.total_timeouts = 0;
            self.cumulative_latency_ms = 0.0;
        }
    }

    // ============================================================================
    // Late Correction
    // ============================================================================

    /// Apply a late correction signal to a context window.
    ///
    /// The correction is applied as a weighted adjustment to the last N elements
    /// of the context window, where N = max(1, window_len / 4).
    ///
    /// Formula: `window[i] += signal * decay_factor`
    /// where `decay_factor = 1.0 - (i - start) / correction_span`
    ///
    /// # Arguments
    /// * `context_window` - Mutable slice of f32 values
    /// * `signal` - Correction signal value in [-1.0, 1.0]
    /// * `delay_ms` - Delay in milliseconds (used for decay calculation)
    ///
    /// # Errors
    /// * `AsyncSteeringError::WindowTooSmall` if window has fewer than 4 elements
    pub fn apply_late_correction(
        context_window: &mut [f32],
        signal: f32,
        delay_ms: u64,
    ) -> Result<(), AsyncSteeringError> {
        if context_window.len() < 4 {
            return Err(AsyncSteeringError::WindowTooSmall);
        }

        // Correction span: last 25% of window
        let span = context_window.len() / 4;
        let start = context_window.len() - span;

        // Time-based decay: older signals have less impact
        // decay = max(0.1, 1.0 - delay_ms / 1000.0)
        let time_decay = (1.0 - (delay_ms as f32) / 1000.0).max(0.1);

        for (i, val) in context_window.iter_mut().enumerate().skip(start) {
            let position = i - start;
            let positional_decay = 1.0 - (position as f32) / (span as f32);
            let adjustment = signal * time_decay * positional_decay;
            *val += adjustment;
        }

        Ok(())
    }

    // ============================================================================
    // Tests
    // ============================================================================

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_steering_signal_creation() {
            let signal = SteeringSignal::new(0.5, 100, "fed1".to_string(), 1).unwrap();
            assert_eq!(signal.value, 0.5);
            assert_eq!(signal.delay_ms, 100);
            assert_eq!(signal.source, "fed1");
            assert_eq!(signal.seq, 1);
        }

        #[test]
        fn test_steering_signal_out_of_bounds() {
            let result = SteeringSignal::new(1.5, 100, "fed1".to_string(), 1);
            assert_eq!(result.unwrap_err(), AsyncSteeringError::SignalOutOfBounds);

            let result = SteeringSignal::new(-1.5, 100, "fed1".to_string(), 1);
            assert_eq!(result.unwrap_err(), AsyncSteeringError::SignalOutOfBounds);
        }

        #[test]
        fn test_steering_signal_boundary_values() {
            // Boundary values should be valid
            SteeringSignal::new(1.0, 0, "s".to_string(), 0).unwrap();
            SteeringSignal::new(-1.0, 0, "s".to_string(), 0).unwrap();
            SteeringSignal::new(0.0, 0, "s".to_string(), 0).unwrap();
        }

        #[test]
        fn test_mock_channel_send_recv() {
            let mut channel = AsyncSteeringChannelMock::new(10);
            let signal = SteeringSignal::new(0.5, 100, "fed1".to_string(), 1).unwrap();
            channel.send(signal).unwrap();

            assert_eq!(channel.len(), 1);
            let received = channel.recv().unwrap();
            assert_eq!(received.value, 0.5);
            assert!(channel.is_empty());
        }

        #[test]
        fn test_mock_channel_capacity() {
            let mut channel = AsyncSteeringChannelMock::new(2);
            channel
                .send(SteeringSignal::new(0.1, 0, "s".to_string(), 0).unwrap())
                .unwrap();
            channel
                .send(SteeringSignal::new(0.2, 0, "s".to_string(), 1).unwrap())
                .unwrap();

            // Third send should fail (capacity = 2)
            let result = channel.send(SteeringSignal::new(0.3, 0, "s".to_string(), 2).unwrap());
            assert_eq!(result.unwrap_err(), AsyncSteeringError::ChannelClosed);
        }

        #[test]
        fn test_mock_channel_fifo_order() {
            let mut channel = AsyncSteeringChannelMock::new(10);
            for i in 0..5 {
                channel
                    .send(SteeringSignal::new(i as f32 * 0.1, 0, "s".to_string(), i).unwrap())
                    .unwrap();
            }

            for i in 0..5 {
                let signal = channel.recv().unwrap();
                assert_eq!(signal.seq, i);
            }
        }

        #[test]
        fn test_apply_late_correction_basic() {
            let mut window = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
            apply_late_correction(&mut window, 0.5, 0).unwrap();

            // Last 25% (indices 6-7) should be modified
            assert!(window[6] > 7.0, "Index 6 should increase");
            assert!(window[7] > 8.0, "Index 7 should increase");
            // First 75% should be unchanged
            assert_eq!(window[0], 1.0);
            assert_eq!(window[3], 4.0);
        }

        #[test]
        fn test_apply_late_correction_negative_signal() {
            let mut window = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
            apply_late_correction(&mut window, -0.5, 0).unwrap();

            assert!(window[6] < 7.0, "Index 6 should decrease");
            assert!(window[7] < 8.0, "Index 7 should decrease");
        }

        #[test]
        fn test_apply_late_correction_delay_decay() {
            let mut window1 = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
            let mut window2 = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

            apply_late_correction(&mut window1, 0.5, 0).unwrap(); // No delay
            apply_late_correction(&mut window2, 0.5, 500).unwrap(); // 500ms delay

            // window1 should have larger correction than window2
            let diff1 = window1[7] - 8.0;
            let diff2 = window2[7] - 8.0;
            assert!(
                diff1 > diff2,
                "No-delay correction ({:.4}) should exceed delayed ({:.4})",
                diff1,
                diff2
            );
        }

        #[test]
        fn test_apply_late_correction_window_too_small() {
            let mut window = vec![1.0, 2.0, 3.0];
            let result = apply_late_correction(&mut window, 0.5, 0);
            assert_eq!(result.unwrap_err(), AsyncSteeringError::WindowTooSmall);
        }

        #[test]
        fn test_apply_late_correction_exactly_four() {
            let mut window = vec![1.0, 2.0, 3.0, 4.0];
            let result = apply_late_correction(&mut window, 0.5, 0);
            assert!(result.is_ok());
            // Last element (index 3) should be modified
            assert!(window[3] > 4.0);
        }

        #[test]
        fn test_error_display() {
            assert!(AsyncSteeringError::ChannelClosed.to_string().contains("channel"));
            assert!(AsyncSteeringError::WindowTooSmall.to_string().contains("window"));
            assert!(AsyncSteeringError::SignalOutOfBounds.to_string().contains("signal"));
        }

        #[test]
        fn test_positional_decay() {
            let mut window = vec![0.0f32; 16];
            apply_late_correction(&mut window, 1.0, 0).unwrap();

            // Correction span = indices 12-15
            // Positional decay: index 12 = 1.0, index 15 = 0.0 (approx)
            // So index 12 should have larger correction than index 15
            assert!(
                window[12] > window[15],
                "Earlier position should have larger correction"
            );
        }

        #[test]
        fn test_try_send_success() {
            let mut channel = AsyncSteeringChannelMock::new(2);
            let signal = SteeringSignal::new(0.5, 100, "fed1".to_string(), 1).unwrap();
            assert!(channel.try_send(signal).is_ok());
            assert_eq!(channel.len(), 1);
        }

        #[test]
        fn test_try_send_backpressure() {
            let mut channel = AsyncSteeringChannelMock::new(1);
            channel
                .try_send(SteeringSignal::new(0.1, 0, "s".to_string(), 0).unwrap())
                .unwrap();
            let result = channel.try_send(SteeringSignal::new(0.2, 0, "s".to_string(), 1).unwrap());
            assert_eq!(result.unwrap_err(), AsyncSteeringError::Backpressure);
        }

        #[test]
        fn test_metrics_default() {
            let metrics = SteeringMetrics::default();
            assert_eq!(metrics.total_sent, 0);
            assert_eq!(metrics.total_received, 0);
            assert_eq!(metrics.total_dropped, 0);
            assert_eq!(metrics.total_timeouts, 0);
            assert_eq!(metrics.avg_latency_ms(), 0.0);
            assert_eq!(metrics.drop_rate(), 0.0);
        }

        #[test]
        fn test_metrics_record_send() {
            let mut metrics = SteeringMetrics::default();
            metrics.record_send(10.0);
            metrics.record_send(20.0);
            assert_eq!(metrics.total_sent, 2);
            assert_eq!(metrics.avg_latency_ms(), 15.0);
        }

        #[test]
        fn test_metrics_record_drop() {
            let mut metrics = SteeringMetrics::default();
            metrics.record_send(10.0);
            metrics.record_drop();
            assert_eq!(metrics.total_dropped, 1);
            assert!((metrics.drop_rate() - 0.5).abs() < f64::EPSILON);
        }

        #[test]
        fn test_metrics_record_timeout() {
            let mut metrics = SteeringMetrics::default();
            metrics.record_timeout();
            assert_eq!(metrics.total_timeouts, 1);
        }

        #[test]
        fn test_metrics_reset() {
            let mut metrics = SteeringMetrics::default();
            metrics.record_send(10.0);
            metrics.record_drop();
            metrics.reset();
            assert_eq!(metrics.total_sent, 0);
            assert_eq!(metrics.total_dropped, 0);
            assert_eq!(metrics.cumulative_latency_ms, 0.0);
        }

        #[test]
        fn test_backpressure_display() {
            assert!(AsyncSteeringError::Backpressure.to_string().contains("backpressure"));
        }

        #[test]
        fn test_timeout_display() {
            assert!(AsyncSteeringError::Timeout.to_string().contains("timeout"));
        }
    }
}

pub use internal::{
    apply_late_correction, AsyncSteeringChannelMock, AsyncSteeringError, SteeringMetrics,
    SteeringSignal,
};

