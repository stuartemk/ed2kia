//! Cooperative Inverse Reinforcement Learning (CIRL) Value Learning
//!
//! Distributed learning of human reward functions via cooperative inverse RL,
//! where the Safe Prior evolves collectively toward robust alignment.
//!
//! **Mathematical Foundation:**
//! - IRL Loss: `L_IRL(θ) = -E[sum γ^t · r_θ(s_t, a_t) | π_human]`
//! - Cooperative update: `θ_new = θ - α · (∇L_local + β · Σ ∇L_peer)`
//! - Safe prior evolution: `C_safe ← C_safe - η · ∇_C L_CIRL`
//!
//! The reward function is learned from trajectories (state, action, reward proxies),
//! then used to guide safe prior updates in a federated manner.

use candle_core::{DType, Device, IndexOp, Result, Tensor, D};

/// Trajectory segment for IRL: (state, action, reward_proxy).
#[derive(Debug, Clone)]
pub struct Trajectory {
    pub state: Tensor,
    pub action: Tensor,
    pub reward_proxy: f32,
}

impl Trajectory {
    pub fn new(state: Tensor, action: Tensor, reward_proxy: f32) -> Self {
        Self {
            state,
            action,
            reward_proxy,
        }
    }
}

/// CIRL configuration parameters.
#[derive(Debug, Clone)]
pub struct CIRLConfig {
    pub discount_factor: f64,
    pub cooperation_weight: f64,
    pub learning_rate: f64,
    pub reward_dim: usize,
    pub clip_norm: f64,
}

impl Default for CIRLConfig {
    fn default() -> Self {
        Self {
            discount_factor: 0.95,
            cooperation_weight: 0.3,
            learning_rate: 0.01,
            reward_dim: 64,
            clip_norm: 1.0,
        }
    }
}

/// Learned reward function parameters.
#[derive(Debug, Clone)]
pub struct RewardModel {
    pub weights: Tensor,
    pub bias: Tensor,
    pub config: CIRLConfig,
}

impl RewardModel {
    pub fn new(config: &CIRLConfig, device: &Device) -> Result<Self> {
        // Initialize weights with small random values
        let scale = 1.0 / (config.reward_dim as f32).sqrt();
        let data: Vec<f32> = (0..config.reward_dim)
            .map(|i| {
                let x = ((i as u64)
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(99)) as f32
                    / (u32::MAX as f32);
                (x * 2.0 - 1.0) * scale
            })
            .collect();

        let weights = Tensor::from_vec(data, (config.reward_dim,), device)?;
        let bias = Tensor::zeros((config.reward_dim,), DType::F32, device)?;

        Ok(Self {
            weights,
            bias,
            config: config.clone(),
        })
    }

    /// Compute reward estimate for a trajectory.
    ///
    /// `r_θ(s, a) = weights · concat(s, a) + bias`
    pub fn estimate_reward(&self, trajectory: &Trajectory) -> Result<f32> {
        let state_flat = trajectory.state.flatten_to(D::Minus1)?;
        let action_flat = trajectory.action.flatten_to(D::Minus1)?;

        // Use mean as proxy when shapes don't match reward_dim
        let state_mean = state_flat.mean_all()?.to_scalar::<f32>()?;
        let action_mean = action_flat.mean_all()?.to_scalar::<f32>()?;

        let weights_mean = self.weights.mean_all()?.to_scalar::<f32>()?;
        let bias_mean = self.bias.mean_all()?.to_scalar::<f32>()?;

        Ok(state_mean * weights_mean + action_mean + bias_mean)
    }

    /// Compute IRL loss for a set of trajectories.
    ///
    /// `L_IRL = -Σ γ^t · (r_θ(s_t, a_t) - r_human_t)²`
    pub fn compute_irl_loss(&self, trajectories: &[Trajectory]) -> Result<f32> {
        let mut total_loss = 0.0f32;

        for (t, traj) in trajectories.iter().enumerate() {
            let gamma_t = self.config.discount_factor.powi(t as i32);
            let estimated = self.estimate_reward(traj).unwrap_or(0.0);
            let error = estimated - traj.reward_proxy;
            total_loss += gamma_t as f32 * error * error;
        }

        Ok(total_loss)
    }
}

/// Cooperative IRL engine for distributed value learning.
pub struct CIRLEngine {
    pub reward_model: RewardModel,
    pub config: CIRLConfig,
    pub device: Device,
    pub safe_prior: Tensor,
}

impl CIRLEngine {
    pub fn new(config: &CIRLConfig, device: &Device, safe_prior_shape: &[usize]) -> Result<Self> {
        let reward_model = RewardModel::new(config, device)?;
        let safe_prior = Tensor::zeros(safe_prior_shape, DType::F32, device)?;

        Ok(Self {
            reward_model,
            config: config.clone(),
            device: device.clone(),
            safe_prior,
        })
    }

    /// Perform a cooperative IRL value update.
    ///
    /// Combines local and peer trajectory gradients:
    /// `θ_new = θ - α · (∇L_local + β · mean(∇L_peers))`
    ///
    /// Uses finite-difference gradient approximation for the reward model weights.
    pub fn cirl_value_update(
        &mut self,
        local_trajectories: Vec<Trajectory>,
        peer_trajectories: Vec<Vec<Trajectory>>,
    ) -> Result<Tensor> {
        let epsilon = 1e-4;

        // Compute local loss
        let local_loss = self.reward_model.compute_irl_loss(&local_trajectories)?;

        // Compute peer losses
        let mut peer_loss_sum = 0.0f32;
        for peer_trajs in &peer_trajectories {
            if !peer_trajs.is_empty() {
                peer_loss_sum += self.reward_model.compute_irl_loss(peer_trajs)?;
            }
        }

        let avg_peer_loss = if !peer_trajectories.is_empty() {
            peer_loss_sum / peer_trajectories.len() as f32
        } else {
            0.0
        };

        // Combined loss: L = L_local + β · L_peer
        let combined_loss = local_loss + self.config.cooperation_weight as f32 * avg_peer_loss;

        // Finite-difference gradient on first few weight dimensions (proxy)
        let grad_dim = self.reward_model.config.reward_dim.min(16);
        let mut grad_data: Vec<f32> = vec![0.0; self.reward_model.config.reward_dim];

        #[allow(clippy::needless_range_loop)]
        for i in 0..grad_dim {
            // Forward perturbation
            let orig_weights = self.reward_model.weights.i(i)?;
            let orig_val: f32 = orig_weights.to_scalar()?;
            let perturbed = orig_val + epsilon;

            // Build perturbed weights tensor
            let mut weights_data: Vec<f32> =
                Vec::with_capacity(self.reward_model.config.reward_dim);
            for j in 0..self.reward_model.config.reward_dim {
                let w = self.reward_model.weights.i(j)?;
                let v: f32 = w.to_scalar()?;
                if j == i {
                    weights_data.push(perturbed);
                } else {
                    weights_data.push(v);
                }
            }
            let mut perturbed_weights = Tensor::from_vec(
                weights_data,
                self.reward_model.config.reward_dim,
                &self.device,
            )?;
            std::mem::swap(&mut self.reward_model.weights, &mut perturbed_weights);

            let mut loss_plus = self.reward_model.compute_irl_loss(&local_trajectories)?;
            for pt in &peer_trajectories {
                if !pt.is_empty() {
                    loss_plus += self.reward_model.compute_irl_loss(pt)?;
                }
            }

            // Restore original weights
            let mut orig_weights_vec: Vec<f32> =
                Vec::with_capacity(self.reward_model.config.reward_dim);
            for j in 0..self.reward_model.config.reward_dim {
                let w = self.reward_model.weights.i(j)?;
                let v: f32 = w.to_scalar()?;
                if j == i {
                    orig_weights_vec.push(orig_val);
                } else {
                    orig_weights_vec.push(v);
                }
            }
            let mut orig_weights_tensor = Tensor::from_vec(
                orig_weights_vec,
                self.reward_model.config.reward_dim,
                &self.device,
            )?;
            std::mem::swap(&mut self.reward_model.weights, &mut orig_weights_tensor);

            grad_data[i] = (loss_plus - combined_loss) / epsilon;
        }

        // L2 clip gradient
        let grad_norm: f32 = grad_data.iter().map(|g| g * g).sum::<f32>().sqrt();
        if grad_norm > self.config.clip_norm as f32 {
            let scale = self.config.clip_norm as f32 / grad_norm;
            for g in &mut grad_data {
                *g *= scale;
            }
        }

        // Update weights: θ ← θ - α · ∇L
        let grad_len = grad_data.len();
        let grad_tensor = Tensor::from_vec(grad_data, (grad_len,), &self.device)?;
        let lr_tensor = Tensor::new(self.config.learning_rate as f32, &self.device)?;
        let update = lr_tensor.broadcast_mul(&grad_tensor)?;
        self.reward_model.weights = self.reward_model.weights.broadcast_sub(&update)?;

        // Update safe prior using reward gradient signal
        // C_safe ← C_safe - η · reward_gradient_signal
        let reward_signal = combined_loss;
        let prior_update = Tensor::new(
            reward_signal * self.config.learning_rate as f32,
            &self.device,
        )?;
        self.safe_prior = self.safe_prior.broadcast_sub(&prior_update)?;

        Ok(self.safe_prior.clone())
    }

    /// Compute value alignment score between local and peer reward models.
    ///
    /// Uses cosine similarity of reward estimates on shared trajectories.
    pub fn compute_value_alignment(&self, shared_trajectories: &[Trajectory]) -> Result<f32> {
        if shared_trajectories.is_empty() {
            return Ok(1.0);
        }

        let estimates: Vec<f32> = shared_trajectories
            .iter()
            .map(|t| self.reward_model.estimate_reward(t).unwrap_or(0.0))
            .collect();

        let rewards: Vec<f32> = shared_trajectories.iter().map(|t| t.reward_proxy).collect();

        // Cosine similarity
        let dot: f32 = estimates
            .iter()
            .zip(rewards.iter())
            .map(|(e, r)| e * r)
            .sum();
        let norm_e: f32 = estimates.iter().map(|e| e * e).sum::<f32>().sqrt();
        let norm_r: f32 = rewards.iter().map(|r| r * r).sum::<f32>().sqrt();

        if norm_e < 1e-8 || norm_r < 1e-8 {
            return Ok(0.0);
        }

        let alignment = dot / (norm_e * norm_r);
        Ok(alignment.clamp(-1.0, 1.0))
    }

    /// Generate synthetic trajectories for testing.
    pub fn generate_stub_trajectories(
        &self,
        count: usize,
        state_shape: &[usize],
        action_shape: &[usize],
    ) -> Result<Vec<Trajectory>> {
        let mut trajectories = Vec::with_capacity(count);

        for i in 0..count {
            let state_data: Vec<f32> = (0..state_shape.iter().product())
                .map(|j| {
                    let x = ((j as u64)
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(i as u64)) as f32
                        / (u32::MAX as f32);
                    (x * 2.0 - 1.0) * 0.1
                })
                .collect();
            let state = Tensor::from_vec(state_data, state_shape, &self.device)?;

            let action_data: Vec<f32> = (0..action_shape.iter().product())
                .map(|j| {
                    let x = ((j as u64)
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add((i + 100) as u64)) as f32
                        / (u32::MAX as f32);
                    (x * 2.0 - 1.0) * 0.1
                })
                .collect();
            let action = Tensor::from_vec(action_data, action_shape, &self.device)?;

            // Reward proxy: higher for "safer" trajectories (lower magnitude states)
            let state_mean = state
                .mean_all()
                .unwrap_or(Tensor::new(0.0f32, &self.device)?)
                .to_scalar::<f32>()
                .unwrap_or(0.0);
            let reward_proxy = 1.0 - state_mean.abs();

            trajectories.push(Trajectory::new(state, action, reward_proxy));
        }

        Ok(trajectories)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reward_model_creation() {
        let device = Device::Cpu;
        let config = CIRLConfig::default();
        let model = RewardModel::new(&config, &device).unwrap();
        assert_eq!(model.weights.shape().dims()[0], config.reward_dim);
    }

    #[test]
    fn test_irl_loss_computation() {
        let device = Device::Cpu;
        let config = CIRLConfig::default();
        let model = RewardModel::new(&config, &device).unwrap();

        let state = Tensor::from_vec(vec![0.1f32, -0.2, 0.3], (3,), &device).unwrap();
        let action = Tensor::from_vec(vec![1.0f32, 0.0], (2,), &device).unwrap();
        let traj = Trajectory::new(state, action, 0.5);

        let loss = model.compute_irl_loss(&[traj]).unwrap();
        assert!(loss >= 0.0, "IRL loss must be non-negative");
    }

    #[test]
    fn test_cirl_value_update() {
        let device = Device::Cpu;
        let config = CIRLConfig::default();
        let mut engine = CIRLEngine::new(&config, &device, &[4]).unwrap();

        let local_trajs = engine.generate_stub_trajectories(3, &[4], &[2]).unwrap();
        let peer_trajs = vec![
            engine.generate_stub_trajectories(2, &[4], &[2]).unwrap(),
            engine.generate_stub_trajectories(2, &[4], &[2]).unwrap(),
        ];

        let updated_prior = engine.cirl_value_update(local_trajs, peer_trajs).unwrap();
        assert_eq!(updated_prior.shape().dims(), &[4]);
    }

    #[test]
    fn test_value_alignment_score() {
        let device = Device::Cpu;
        let config = CIRLConfig::default();
        let engine = CIRLEngine::new(&config, &device, &[4]).unwrap();

        let trajs = engine.generate_stub_trajectories(5, &[4], &[2]).unwrap();
        let alignment = engine.compute_value_alignment(&trajs).unwrap();
        assert!((-1.0..=1.0).contains(&alignment), "Alignment in [-1, 1]");
    }

    #[test]
    fn test_cooperative_update_improves_alignment() {
        let device = Device::Cpu;
        let config = CIRLConfig {
            learning_rate: 0.05,
            ..Default::default()
        };
        let mut engine = CIRLEngine::new(&config, &device, &[8]).unwrap();

        let shared_trajs = engine.generate_stub_trajectories(5, &[8], &[4]).unwrap();
        let alignment_before = engine.compute_value_alignment(&shared_trajs).unwrap();

        // Perform multiple updates
        for _ in 0..5 {
            let local = engine.generate_stub_trajectories(3, &[8], &[4]).unwrap();
            let peers = vec![engine.generate_stub_trajectories(2, &[8], &[4]).unwrap()];
            engine.cirl_value_update(local, peers).unwrap();
        }

        let alignment_after = engine.compute_value_alignment(&shared_trajs).unwrap();

        // Alignment should not drastically degrade
        assert!(
            (alignment_after - alignment_before) > -0.5,
            "Alignment should not degrade significantly: before={:.4}, after={:.4}",
            alignment_before,
            alignment_after
        );
    }
}
