//! Event-Triggered Controller with Zeno Avoidance for Distributionally Robust CBF.
//!
//! **Sprint 170:** Implements sample-and-hold event-triggered control (ETC) with
//! Wasserstein drift monitoring and Zeno-free guarantees for edge/WASM deployment.
//!
//! **Trigger Conditions:**
//! 1. Barrier degradation: `h(φ̂_{t+1}) < h_margin`
//! 2. Wasserstein drift: `W_p(P_recent, P_safe_prior) > δ`
//! 3. Lyapunov drift: `V_dot > 0` or `V > V_threshold`
//!
//! **Zeno Avoidance:** Hard minimum inter-event time `τ_min` based on
//! Lipschitz bounds of closed-loop dynamics + class-K functions.
//!
//! **Sample-and-Hold:** Between triggers, hold last computed control `u_last`.

use candle_core::{DType, Device, Result, Tensor};

use crate::control_lmi::{compute_tau_min, dr_cbf_trigger};

/// Configuration for the event-triggered controller.
#[derive(Debug, Clone)]
pub struct EventTriggerConfig {
    /// Minimum inter-event time in steps (Zeno avoidance).
    pub tau_min: usize,
    /// Wasserstein drift threshold δ for distribution shift detection.
    pub w_threshold: f32,
    /// CBF safety margin for early triggering.
    pub h_margin: f32,
    /// Lyapunov threshold V_threshold for stability monitoring.
    pub v_threshold: f32,
    /// Wasserstein ambiguity radius for DR-CBF.
    pub wasserstein_epsilon: f32,
    /// Lipschitz constant of barrier function h.
    pub lip_h: f32,
    /// History buffer size for Wasserstein drift estimation.
    pub history_size: usize,
}

impl Default for EventTriggerConfig {
    fn default() -> Self {
        Self {
            tau_min: 5,
            w_threshold: 0.5,
            h_margin: 0.1,
            v_threshold: 1.0,
            wasserstein_epsilon: 0.1,
            lip_h: 1.0,
            history_size: 50,
        }
    }
}

impl EventTriggerConfig {
    /// Create config with custom tau_min.
    pub fn with_tau_min(mut self, tau_min: usize) -> Self {
        self.tau_min = tau_min.max(1);
        self
    }

    /// Create config with custom Wasserstein threshold.
    pub fn with_w_threshold(mut self, threshold: f32) -> Self {
        self.w_threshold = threshold.max(0.0);
        self
    }

    /// Create config with custom safety margin.
    pub fn with_h_margin(mut self, margin: f32) -> Self {
        self.h_margin = margin;
        self
    }

    /// Create config with custom Lyapunov threshold.
    pub fn with_v_threshold(mut self, threshold: f32) -> Self {
        self.v_threshold = threshold;
        self
    }
}

/// Result of an event-triggered control step.
#[derive(Debug, Clone)]
pub struct EventTriggerResult {
    /// Control output for this step.
    pub u: Tensor,
    /// Whether a new control was computed (trigger fired).
    pub triggered: bool,
    /// Trigger reason (if triggered).
    pub trigger_reason: Option<TriggerReason>,
    /// Current step index.
    pub step: usize,
    /// Time since last trigger.
    pub time_since_trigger: usize,
}

impl std::fmt::Display for EventTriggerResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EventTriggerResult(step={}, triggered={}, reason={:?}, dt={})",
            self.step,
            self.triggered,
            self.trigger_reason,
            self.time_since_trigger
        )
    }
}

/// Reason for event trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerReason {
    /// CBF barrier degraded below safety margin.
    CbfDegraded,
    /// Wasserstein drift exceeded threshold.
    WassersteinDrift,
    /// Lyapunov function increasing (instability).
    LyapunovDrift,
    /// Initial trigger (first step).
    Initial,
}

/// Event-Triggered Controller with sample-and-hold and Zeno avoidance.
///
/// Maintains state between triggers: last control input, trigger time,
/// and activation history for Wasserstein drift estimation.
pub struct EventTriggeredController {
    config: EventTriggerConfig,
    last_trigger_step: usize,
    last_u: Option<Tensor>,
    initialized: bool,
    /// Recent activation history for Wasserstein drift estimation.
    recent_activations: Vec<Tensor>,
    /// Safe prior centroid for Wasserstein comparison.
    safe_prior: Option<Tensor>,
    device: Device,
}

impl EventTriggeredController {
    /// Create a new event-triggered controller.
    pub fn new(config: EventTriggerConfig, device: &Device) -> Self {
        Self {
            config,
            last_trigger_step: 0,
            last_u: None,
            initialized: false,
            recent_activations: Vec::new(),
            safe_prior: None,
            device: device.clone(),
        }
    }

    /// Set the safe prior centroid for Wasserstein drift monitoring.
    pub fn set_safe_prior(&mut self, safe_prior: &Tensor) {
        self.safe_prior = Some(safe_prior.clone());
    }

    /// Execute one event-triggered control step.
    ///
    /// Returns the control input (either held from last trigger or newly computed)
    /// and metadata about whether a trigger occurred.
    ///
    /// # Arguments
    /// * `current_step` - Current time step index.
    /// * `h_val` - Current barrier function value.
    /// * `v_dot` - Lyapunov derivative (positive = unstable).
    /// * `v_val` - Current Lyapunov value.
    /// * `phi_hat` - Current Koopman state estimate.
    /// * `compute_new_control` - Closure to compute new control when triggered.
    pub fn step<F>(&mut self, current_step: usize, h_val: f32, v_dot: f32, v_val: f32, phi_hat: &Tensor, compute_new_control: F) -> Result<EventTriggerResult>
    where
        F: FnOnce() -> Result<Tensor>,
    {
        let time_since_trigger = current_step.saturating_sub(self.last_trigger_step);

        // Initial step always triggers
        if !self.initialized {
            self.initialized = true;
            let u_new = compute_new_control()?;
            self.last_u = Some(u_new.clone());
            self.last_trigger_step = current_step;
            self.update_history(phi_hat)?;
            return Ok(EventTriggerResult {
                u: u_new,
                triggered: true,
                trigger_reason: Some(TriggerReason::Initial),
                step: current_step,
                time_since_trigger: 0,
            });
        }

        // Zeno avoidance: enforce minimum inter-event time
        if time_since_trigger < self.config.tau_min {
            return Ok(EventTriggerResult {
                u: self.last_u.clone().unwrap_or_else(|| Tensor::zeros((1,), DType::F32, &self.device).unwrap_or_else(|_| Tensor::zeros((1,), DType::F32, &Device::Cpu).unwrap())),
                triggered: false,
                trigger_reason: None,
                step: current_step,
                time_since_trigger,
            });
        }

        // Check trigger conditions
        let mut trigger = false;
        let mut reason = None;

        // 1. DR-CBF trigger
        if dr_cbf_trigger(
            h_val,
            self.config.wasserstein_epsilon,
            self.config.lip_h,
            self.config.h_margin,
        ) {
            trigger = true;
            reason = Some(TriggerReason::CbfDegraded);
        }

        // 2. Wasserstein drift trigger
        if !trigger {
            if let Some(w_drift) = self.compute_wasserstein_drift(phi_hat)? {
                if w_drift > self.config.w_threshold {
                    trigger = true;
                    reason = Some(TriggerReason::WassersteinDrift);
                }
            }
        }

        // 3. Lyapunov drift trigger
        if !trigger {
            if v_dot > 0.0 || v_val > self.config.v_threshold {
                trigger = true;
                reason = Some(TriggerReason::LyapunovDrift);
            }
        }

        if trigger {
            let u_new = compute_new_control()?;
            self.last_u = Some(u_new.clone());
            self.last_trigger_step = current_step;
            self.update_history(phi_hat)?;
            Ok(EventTriggerResult {
                u: u_new,
                triggered: true,
                trigger_reason: reason,
                step: current_step,
                time_since_trigger,
            })
        } else {
            // Sample-and-hold: return last control
            Ok(EventTriggerResult {
                u: self.last_u.clone().unwrap_or_else(|| Tensor::zeros((1,), DType::F32, &self.device).unwrap_or_else(|_| Tensor::zeros((1,), DType::F32, &Device::Cpu).unwrap())),
                triggered: false,
                trigger_reason: None,
                step: current_step,
                time_since_trigger,
            })
        }
    }

    /// Compute Wasserstein drift between recent activations and safe prior.
    ///
    /// Uses mean distance as a proxy for Wasserstein-1 distance:
    /// `W_1 ≈ ||μ_recent - μ_prior||`
    ///
    /// # Returns
    /// `Some(drift)` if safe prior is set and history is available, `None` otherwise.
    pub fn compute_wasserstein_drift(&self, current: &Tensor) -> Result<Option<f32>> {
        let prior = match &self.safe_prior {
            Some(p) => p,
            None => return Ok(None),
        };

        if self.recent_activations.is_empty() {
            return Ok(None);
        }

        // Compute mean of recent activations
        let mut sum: Option<Tensor> = None;
        for act in &self.recent_activations {
            sum = match sum {
                Some(s) => Some(s.add(act)?),
                None => Some(act.clone()),
            };
        }

        let mean = match sum {
            Some(s) => s.broadcast_div(&Tensor::new(
                self.recent_activations.len() as f32,
                &self.device,
            )?)?,
            None => return Ok(None),
        };

        // Combined centroid: mean of recent + current
        let centroid = mean.add(current)?.broadcast_div(&Tensor::new(
            2.0,
            &self.device,
        )?)?;

        // W_1 proxy: ||centroid - prior||
        let diff = centroid.broadcast_sub(prior)?;
        let drift = diff.sqr()?.sum_all()?.to_scalar::<f32>()?.sqrt() as f32;
        Ok(Some(drift))
    }

    /// Update activation history buffer.
    fn update_history(&mut self, phi: &Tensor) -> Result<()> {
        self.recent_activations.push(phi.clone());
        while self.recent_activations.len() > self.config.history_size {
            self.recent_activations.remove(0);
        }
        Ok(())
    }

    /// Compute τ_min for Zeno avoidance based on current state.
    ///
    /// # Arguments
    /// * `h_min` - Minimum safe barrier value.
    /// * `l_f` - Lipschitz constant of nominal dynamics.
    /// * `l_g` - Lipschitz constant of control matrix.
    /// * `u_max` - Maximum control norm.
    pub fn compute_tau_min(&self, h_min: f32, l_f: f32, l_g: f32, u_max: f32) -> f32 {
        compute_tau_min(1.0, h_min, l_f, l_g, u_max, 0.1)
    }

    /// Get statistics about trigger history.
    pub fn stats(&self) -> (usize, bool) {
        (self.last_trigger_step, self.initialized)
    }

    /// Reset controller state.
    pub fn reset(&mut self) {
        self.last_trigger_step = 0;
        self.last_u = None;
        self.initialized = false;
        self.recent_activations.clear();
    }
}

/// Compute event-triggered trajectory statistics.
///
/// # Arguments
/// * `results` - Vector of EventTriggerResult from a simulation.
///
/// # Returns
/// `(total_triggers, total_steps, trigger_rate)`
pub fn compute_trigger_stats(results: &[EventTriggerResult]) -> (usize, usize, f32) {
    let total_steps = results.len();
    let total_triggers = results.iter().filter(|r| r.triggered).count();
    let trigger_rate = if total_steps > 0 {
        total_triggers as f32 / total_steps as f32
    } else {
        0.0
    };
    (total_triggers, total_steps, trigger_rate)
}

/// Verify Zeno avoidance guarantee.
///
/// Returns `true` if minimum inter-event time is respected.
pub fn verify_zeno_avoidance(results: &[EventTriggerResult], tau_min: usize) -> bool {
    let mut last_trigger: Option<usize> = None;
    for r in results {
        if r.triggered {
            if let Some(last) = last_trigger {
                if (r.step - last) < tau_min {
                    return false;
                }
            }
            last_trigger = Some(r.step);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tensor(dims: &[usize], seed: f32, device: &Device) -> Result<Tensor> {
        let n: usize = dims.iter().product();
        let data: Vec<f32> = (0..n).map(|i| seed + i as f32 * 0.01).collect();
        Tensor::from_vec(data, dims, device)
    }

    #[test]
    fn test_event_trigger_config_default() {
        let cfg = EventTriggerConfig::default();
        assert_eq!(cfg.tau_min, 5);
        assert!(cfg.w_threshold > 0.0);
        assert!(cfg.h_margin > 0.0);
    }

    #[test]
    fn test_event_trigger_config_custom() {
        let cfg = EventTriggerConfig::default()
            .with_tau_min(10)
            .with_w_threshold(1.0)
            .with_h_margin(0.5);
        assert_eq!(cfg.tau_min, 10);
        assert!((cfg.w_threshold - 1.0).abs() < 1e-6);
        assert!((cfg.h_margin - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_event_trigger_initial() -> Result<()> {
        let cfg = EventTriggerConfig::default();
        let mut ctrl = EventTriggeredController::new(cfg, &Device::Cpu);
        let phi = make_tensor(&[10], 1.0, &Device::Cpu)?;

        let result = ctrl.step(0, 1.0, -0.1, 0.5, &phi, || {
            make_tensor(&[5], 0.5, &Device::Cpu)
        })?;

        assert!(result.triggered);
        assert_eq!(result.trigger_reason, Some(TriggerReason::Initial));
        assert_eq!(result.step, 0);
        Ok(())
    }

    #[test]
    fn test_zeno_avoidance() -> Result<()> {
        let cfg = EventTriggerConfig::default().with_tau_min(5);
        let mut ctrl = EventTriggeredController::new(cfg, &Device::Cpu);
        let phi = make_tensor(&[10], 1.0, &Device::Cpu)?;

        // Initial trigger
        ctrl.step(0, 1.0, -0.1, 0.5, &phi, || make_tensor(&[5], 0.5, &Device::Cpu))?;

        // Steps 1-4 should be blocked by Zeno avoidance
        for step in 1..5 {
            let result = ctrl.step(step, 1.0, -0.1, 0.5, &phi, || {
                make_tensor(&[5], 0.5, &Device::Cpu)
            })?;
            assert!(!result.triggered, "Step {} triggered despite Zeno avoidance", step);
            assert_eq!(result.time_since_trigger, step);
        }

        // Step 5 should be allowed to trigger if condition met
        let result = ctrl.step(5, 0.01, 0.1, 2.0, &phi, || {
            make_tensor(&[5], 0.5, &Device::Cpu)
        })?;
        assert!(result.triggered, "Step 5 should be allowed to trigger");
        Ok(())
    }

    #[test]
    fn test_cbf_trigger() -> Result<()> {
        let cfg = EventTriggerConfig::default()
            .with_tau_min(1)
            .with_h_margin(0.1);
        let mut ctrl = EventTriggeredController::new(cfg, &Device::Cpu);
        let phi = make_tensor(&[10], 1.0, &Device::Cpu)?;

        // Initial trigger
        ctrl.step(0, 1.0, -0.1, 0.5, &phi, || make_tensor(&[5], 0.5, &Device::Cpu))?;

        // Safe state: no trigger
        let result = ctrl.step(1, 1.0, -0.1, 0.5, &phi, || {
            make_tensor(&[5], 0.5, &Device::Cpu)
        })?;
        assert!(!result.triggered);

        // Unsafe state: h_val < h_margin → trigger
        let result = ctrl.step(2, 0.01, 0.1, 0.5, &phi, || {
            make_tensor(&[5], 0.5, &Device::Cpu)
        })?;
        assert!(result.triggered);
        assert_eq!(result.trigger_reason, Some(TriggerReason::CbfDegraded));
        Ok(())
    }

    #[test]
    fn test_lyapunov_trigger() -> Result<()> {
        let cfg = EventTriggerConfig::default().with_tau_min(1);
        let mut ctrl = EventTriggeredController::new(cfg, &Device::Cpu);
        let phi = make_tensor(&[10], 1.0, &Device::Cpu)?;

        // Initial trigger
        ctrl.step(0, 1.0, -0.1, 0.5, &phi, || make_tensor(&[5], 0.5, &Device::Cpu))?;

        // Stable: v_dot < 0, v_val < threshold → no trigger
        let result = ctrl.step(1, 1.0, -0.1, 0.5, &phi, || {
            make_tensor(&[5], 0.5, &Device::Cpu)
        })?;
        assert!(!result.triggered);

        // Unstable: v_dot > 0 → trigger
        let result = ctrl.step(2, 1.0, 0.5, 0.5, &phi, || {
            make_tensor(&[5], 0.5, &Device::Cpu)
        })?;
        assert!(result.triggered);
        assert_eq!(result.trigger_reason, Some(TriggerReason::LyapunovDrift));
        Ok(())
    }

    #[test]
    fn test_sample_and_hold() -> Result<()> {
        let cfg = EventTriggerConfig::default().with_tau_min(1);
        let mut ctrl = EventTriggeredController::new(cfg, &Device::Cpu);
        let phi = make_tensor(&[10], 1.0, &Device::Cpu)?;

        // Initial trigger with known control
        let u_init = make_tensor(&[5], 2.0, &Device::Cpu)?;
        ctrl.step(0, 1.0, -0.1, 0.5, &phi, || Ok(u_init.clone()))?;

        // Safe step: should hold last control
        let result = ctrl.step(1, 1.0, -0.1, 0.5, &phi, || {
            make_tensor(&[5], 99.0, &Device::Cpu) // Different control
        })?;
        assert!(!result.triggered);

        // Verify held control matches initial (not the new one)
        let held = result.u.to_vec1::<f32>()?;
        let init = u_init.to_vec1::<f32>()?;
        for (h, i) in held.iter().zip(init.iter()) {
            assert!((h - i).abs() < 1e-5);
        }
        Ok(())
    }

    #[test]
    fn test_compute_trigger_stats() {
        let results: Vec<EventTriggerResult> = (0..100)
            .map(|i| EventTriggerResult {
                u: Tensor::zeros((1,), DType::F32, &Device::Cpu).unwrap(),
                triggered: i % 10 == 0,
                trigger_reason: None,
                step: i,
                time_since_trigger: i % 10,
            })
            .collect();

        let (triggers, steps, rate) = compute_trigger_stats(&results);
        assert_eq!(triggers, 10);
        assert_eq!(steps, 100);
        assert!((rate - 0.1).abs() < 1e-6);
    }

    #[test]
    fn test_verify_zeno_avoidance_pass() {
        let results: Vec<EventTriggerResult> = vec![
            EventTriggerResult {
                u: Tensor::zeros((1,), DType::F32, &Device::Cpu).unwrap(),
                triggered: true,
                trigger_reason: None,
                step: 0,
                time_since_trigger: 0,
            },
            EventTriggerResult {
                u: Tensor::zeros((1,), DType::F32, &Device::Cpu).unwrap(),
                triggered: false,
                trigger_reason: None,
                step: 1,
                time_since_trigger: 1,
            },
            EventTriggerResult {
                u: Tensor::zeros((1,), DType::F32, &Device::Cpu).unwrap(),
                triggered: true,
                trigger_reason: None,
                step: 5,
                time_since_trigger: 5,
            },
        ];
        assert!(verify_zeno_avoidance(&results, 3));
    }

    #[test]
    fn test_verify_zeno_avoidance_fail() {
        let results: Vec<EventTriggerResult> = vec![
            EventTriggerResult {
                u: Tensor::zeros((1,), DType::F32, &Device::Cpu).unwrap(),
                triggered: true,
                trigger_reason: None,
                step: 0,
                time_since_trigger: 0,
            },
            EventTriggerResult {
                u: Tensor::zeros((1,), DType::F32, &Device::Cpu).unwrap(),
                triggered: true,
                trigger_reason: None,
                step: 1,
                time_since_trigger: 1,
            },
        ];
        assert!(!verify_zeno_avoidance(&results, 3));
    }

    #[test]
    fn test_controller_reset() -> Result<()> {
        let cfg = EventTriggerConfig::default();
        let mut ctrl = EventTriggeredController::new(cfg, &Device::Cpu);
        let phi = make_tensor(&[10], 1.0, &Device::Cpu)?;

        ctrl.step(0, 1.0, -0.1, 0.5, &phi, || make_tensor(&[5], 0.5, &Device::Cpu))?;
        assert!(ctrl.stats().1); // initialized

        ctrl.reset();
        assert!(!ctrl.stats().1); // not initialized

        // After reset, step 0 should trigger as initial
        let result = ctrl.step(0, 1.0, -0.1, 0.5, &phi, || {
            make_tensor(&[5], 0.5, &Device::Cpu)
        })?;
        assert!(result.triggered);
        assert_eq!(result.trigger_reason, Some(TriggerReason::Initial));
        Ok(())
    }

    #[test]
    fn test_event_trigger_result_display() -> Result<()> {
        let result = EventTriggerResult {
            u: Tensor::zeros((1,), DType::F32, &Device::Cpu)?,
            triggered: true,
            trigger_reason: Some(TriggerReason::CbfDegraded),
            step: 42,
            time_since_trigger: 5,
        };
        let display = format!("{}", result);
        assert!(display.contains("step=42"));
        assert!(display.contains("triggered=true"));
        assert!(display.contains("dt=5"));
        Ok(())
    }

    #[test]
    fn test_efficiency_simulation() -> Result<()> {
        // Simulate 1000 steps with safe trajectory — expect <20% triggers
        let cfg = EventTriggerConfig::default().with_tau_min(5);
        let mut ctrl = EventTriggeredController::new(cfg, &Device::Cpu);
        let phi = make_tensor(&[10], 1.0, &Device::Cpu)?;

        let mut results = Vec::new();
        for step in 0..1000 {
            let result = ctrl.step(step, 1.0, -0.1, 0.5, &phi, || {
                make_tensor(&[5], 0.5, &Device::Cpu)
            })?;
            results.push(result);
        }

        let (triggers, steps, rate) = compute_trigger_stats(&results);
        assert_eq!(steps, 1000);
        // Only initial trigger expected for safe trajectory
        assert!(rate < 0.20, "Trigger rate {} should be < 20%", rate);
        assert!(triggers <= 1, "Expected at most 1 trigger for safe trajectory, got {}", triggers);
        Ok(())
    }

    #[test]
    fn test_drift_gradual_simulation() -> Result<()> {
        // Simulate 500 steps with gradual drift — expect triggers as h decreases
        let cfg = EventTriggerConfig::default()
            .with_tau_min(3)
            .with_h_margin(0.1);
        let mut ctrl = EventTriggeredController::new(cfg, &Device::Cpu);
        let phi = make_tensor(&[10], 1.0, &Device::Cpu)?;

        let mut results = Vec::new();
        for step in 0..500 {
            // Gradual h degradation
            let h_val = 1.0 - step as f32 * 0.002;
            let result = ctrl.step(step, h_val, -0.05, 0.5, &phi, || {
                make_tensor(&[5], 0.5, &Device::Cpu)
            })?;
            results.push(result);
        }

        let (triggers, _steps, rate) = compute_trigger_stats(&results);
        assert!(triggers > 0, "Expected triggers for degrading h");
        assert!(
            rate < 0.50,
            "Trigger rate {} should be < 50% with tau_min=3",
            rate
        );
        assert!(
            verify_zeno_avoidance(&results, 3),
            "Zeno avoidance violated"
        );
        Ok(())
    }
}
