//! Universal Symbiosis Protocol (USP) — Message-based protocol for planetary altruistic mesh.
//!
//! Implements the final coordination layer for Noosfera Kernel P2P interoperability:
//! - **Symbiotic Handshake:** Trust-based node onboarding with capability exchange.
//! - **Symbiotic State Propagation:** Gossip of antibodies, safe priors, and VFE gradients.
//! - **Protocol Compliance Verification:** Formal verification of USP invariants.
//!
//! # Mathematical Foundation
//!
//! **Handshake Trust:**
//! ```text
//! Trust(A,B) = σ( α·sim(cap_A, cap_B) + β·reputation_A + γ·reputation_B - δ·threat_score )
//! ```
//!
//! **Propagation Convergence:**
//! ```text
//! Δstate_i(t+1) = λ · Σ_{j∈neighbors} w_ij · (state_j - state_i) - ε · ∇VFE(state_i)
//! ```
//!
//! **Compliance Score:**
//! ```text
//! Compliance = (1 - KL(prior || safe_prior)) · (1 - Byzantine_ratio) · coherence_factor
//! ```

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for Universal Symbiosis Protocol operations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UspConfig {
    /// Handshake trust weight for capability similarity.
    pub alpha: f64,
    /// Handshake trust weight for local reputation.
    pub beta: f64,
    /// Handshake trust weight for peer reputation.
    pub gamma: f64,
    /// Handshake trust weight for threat penalty.
    pub delta: f64,
    /// Propagation learning rate.
    pub propagation_lr: f64,
    /// VFE gradient weight in propagation.
    pub vfe_gradient_weight: f64,
    /// Maximum propagation hops before TTL expiry.
    pub max_hops: u32,
    /// Compliance tolerance threshold.
    pub compliance_tolerance: f64,
    /// Seed for deterministic randomness.
    pub seed: u64,
}

impl Default for UspConfig {
    fn default() -> Self {
        Self {
            alpha: 0.4,
            beta: 0.25,
            gamma: 0.25,
            delta: 1.5,
            propagation_lr: 0.1,
            vfe_gradient_weight: 0.05,
            max_hops: 16,
            compliance_tolerance: 0.9,
            seed: 42,
        }
    }
}

impl UspConfig {
    /// Builder: custom alpha.
    pub fn with_alpha(mut self, alpha: f64) -> Self {
        self.alpha = alpha.max(0.0).min(1.0);
        self
    }

    /// Builder: custom beta.
    pub fn with_beta(mut self, beta: f64) -> Self {
        self.beta = beta.max(0.0).min(1.0);
        self
    }

    /// Builder: custom gamma.
    pub fn with_gamma(mut self, gamma: f64) -> Self {
        self.gamma = gamma.max(0.0).min(1.0);
        self
    }

    /// Builder: custom delta.
    pub fn with_delta(mut self, delta: f64) -> Self {
        self.delta = delta.max(0.0);
        self
    }

    /// Builder: custom propagation learning rate.
    pub fn with_propagation_lr(mut self, lr: f64) -> Self {
        self.propagation_lr = lr.max(0.0001).min(1.0);
        self
    }

    /// Builder: custom VFE gradient weight.
    pub fn with_vfe_gradient_weight(mut self, w: f64) -> Self {
        self.vfe_gradient_weight = w.max(0.0).min(1.0);
        self
    }

    /// Builder: custom max hops.
    pub fn with_max_hops(mut self, hops: u32) -> Self {
        self.max_hops = hops.max(1);
        self
    }

    /// Builder: custom compliance tolerance.
    pub fn with_compliance_tolerance(mut self, tol: f64) -> Self {
        self.compliance_tolerance = tol.max(0.0).min(1.0);
        self
    }

    /// Builder: custom seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Fast configuration for testing.
    pub fn fast() -> Self {
        Self {
            propagation_lr: 0.2,
            max_hops: 4,
            compliance_tolerance: 0.7,
            ..Self::default()
        }
    }

    /// High-precision configuration.
    pub fn high_precision() -> Self {
        Self {
            propagation_lr: 0.01,
            vfe_gradient_weight: 0.02,
            max_hops: 32,
            compliance_tolerance: 0.99,
            ..Self::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

/// Capability descriptor exchanged during handshake.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capability {
    /// Node type identifier.
    pub node_type: String,
    /// VFE computation capability score.
    pub vfe_capability: f64,
    /// Energy efficiency score.
    pub energy_efficiency: f64,
    /// Symbiosis alignment score.
    pub symbiosis_score: f64,
    /// Coherence temperature.
    pub coherence_temperature: f64,
}

impl Default for Capability {
    fn default() -> Self {
        Self {
            node_type: "standard".to_string(),
            vfe_capability: 0.5,
            energy_efficiency: 0.5,
            symbiosis_score: 0.5,
            coherence_temperature: 1.0,
        }
    }
}

/// Handshake request message.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HandshakeRequest {
    /// Requesting node ID.
    pub requester_id: String,
    /// Requester capabilities.
    pub capabilities: Capability,
    /// Requester reputation score.
    pub reputation: f64,
    /// Timestamp.
    pub timestamp: u64,
}

/// Handshake response message.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HandshakeResponse {
    /// Responding node ID.
    pub responder_id: String,
    /// Computed trust score.
    pub trust_score: f64,
    /// Handshake accepted.
    pub accepted: bool,
    /// Assigned neighbor weight.
    pub neighbor_weight: f64,
    /// Timestamp.
    pub timestamp: u64,
}

/// Symbiotic state payload for propagation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SymbioticState {
    /// Source node ID.
    pub source_id: String,
    /// Influence share distribution.
    pub influence_shares: Vec<f64>,
    /// Variational Free Energy values.
    pub vfe_values: Vec<f64>,
    /// Safe prior distribution.
    pub safe_prior: Vec<f64>,
    /// Antibody distribution (threat detection).
    pub antibodies: Vec<f64>,
    /// Current coherence score.
    pub coherence_score: f64,
    /// Hop count.
    pub hops: u32,
    /// Sequence number for ordering.
    pub sequence: u64,
}

impl Default for SymbioticState {
    fn default() -> Self {
        Self {
            source_id: String::new(),
            influence_shares: Vec::new(),
            vfe_values: Vec::new(),
            safe_prior: Vec::new(),
            antibodies: Vec::new(),
            coherence_score: 0.0,
            hops: 0,
            sequence: 0,
        }
    }
}

/// Propagation message.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropagationMessage {
    /// Origin node ID.
    pub origin_id: String,
    /// Symbiotic state payload.
    pub state: SymbioticState,
    /// Current hop count.
    pub current_hop: u32,
    /// Timestamp.
    pub timestamp: u64,
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

/// Result of a USP handshake operation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HandshakeResult {
    /// Trust score computed during handshake.
    pub trust_score: f64,
    /// Capability similarity.
    pub capability_similarity: f64,
    /// Reputation component.
    pub reputation_component: f64,
    /// Threat penalty.
    pub threat_penalty: f64,
    /// Handshake accepted.
    pub accepted: bool,
    /// Assigned neighbor weight.
    pub neighbor_weight: f64,
}

impl std::fmt::Display for HandshakeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HandshakeResult {{ trust: {:.4}, sim: {:.4}, rep: {:.4}, threat: {:.4}, accepted: {}, weight: {:.4} }}",
            self.trust_score, self.capability_similarity, self.reputation_component,
            self.threat_penalty, self.accepted, self.neighbor_weight
        )
    }
}

/// Result of symbiotic state propagation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropagationResult {
    /// Updated influence shares.
    pub updated_shares: Vec<f64>,
    /// Updated VFE values.
    pub updated_vfe: Vec<f64>,
    /// Propagation delta norm.
    pub delta_norm: f64,
    /// Hops consumed.
    pub hops_consumed: u32,
    /// Convergence detected.
    pub converged: bool,
    /// Number of neighbors processed.
    pub neighbors_processed: usize,
}

impl std::fmt::Display for PropagationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PropagationResult {{ delta: {:.4}, hops: {}, converged: {}, neighbors: {} }}",
            self.delta_norm, self.hops_consumed, self.converged, self.neighbors_processed
        )
    }
}

/// Result of protocol compliance verification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComplianceResult {
    /// Prior divergence from safe prior (KL).
    pub prior_divergence: f64,
    /// Byzantine ratio.
    pub byzantine_ratio: f64,
    /// Coherence factor.
    pub coherence_factor: f64,
    /// Overall compliance score.
    pub compliance_score: f64,
    /// Compliant with tolerance.
    pub compliant: bool,
    /// List of violated invariants.
    pub violated_invariants: Vec<String>,
}

impl std::fmt::Display for ComplianceResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ComplianceResult {{ divergence: {:.4}, byzantine: {:.4}, coherence: {:.4}, score: {:.4}, compliant: {} }}",
            self.prior_divergence, self.byzantine_ratio, self.coherence_factor,
            self.compliance_score, self.compliant
        )
    }
}

// ---------------------------------------------------------------------------
// Random helpers (deterministic LCG)
// ---------------------------------------------------------------------------

fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state
}

fn random_uniform(state: &mut u64) -> f64 {
    (lcg_next(state) >> 11) as f64 / (1u64 << 51) as f64
}

// ---------------------------------------------------------------------------
// Math helpers
// ---------------------------------------------------------------------------

/// Sigmoid function.
pub fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for i in 0..len {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    let denom = (norm_a * norm_b).sqrt();
    if denom < 1e-15 {
        return 0.0;
    }
    (dot / denom).clamp(-1.0, 1.0)
}

/// KL divergence D_KL(P || Q).
pub fn kl_divergence(p: &[f64], q: &[f64]) -> f64 {
    let mut kl = 0.0;
    let len = p.len().min(q.len());
    for i in 0..len {
        if p[i] > 1e-15 && q[i] > 1e-15 {
            kl += p[i] * (p[i] / q[i]).ln();
        }
    }
    kl.max(0.0)
}

/// Shannon entropy.
pub fn shannon_entropy(dist: &[f64]) -> f64 {
    let mut h = 0.0;
    for &p in dist {
        if p > 1e-15 {
            h -= p * p.ln();
        }
    }
    h.max(0.0)
}

/// Euclidean norm of a vector.
pub fn euclidean_norm(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

// ---------------------------------------------------------------------------
// Core Protocol Functions
// ---------------------------------------------------------------------------

/// USP Handshake — Compute trust-based symbiotic handshake between two nodes.
///
/// ```text
/// Trust(A,B) = σ( α·sim(cap_A, cap_B) + β·rep_A + γ·rep_B - δ·threat )
/// ```
///
/// # Parameters
/// - `requester`: Handshake request with capabilities and reputation.
/// - `responder_capabilities`: Responder's capability descriptor.
/// - `responder_reputation`: Responder's reputation score.
/// - `threat_score`: Threat assessment score for the requester.
/// - `config`: USP configuration.
///
/// # Returns
/// `HandshakeResult` with trust score, acceptance decision, and neighbor weight.
pub fn usp_handshake(
    requester: &HandshakeRequest,
    responder_capabilities: &Capability,
    responder_reputation: f64,
    threat_score: f64,
    config: &UspConfig,
) -> HandshakeResult {
    // Compute capability similarity.
    let req_caps = &requester.capabilities;
    let cap_vec_a = vec![
        req_caps.vfe_capability,
        req_caps.energy_efficiency,
        req_caps.symbiosis_score,
        1.0 - req_caps.coherence_temperature.min(1.0),
    ];
    let cap_vec_b = vec![
        responder_capabilities.vfe_capability,
        responder_capabilities.energy_efficiency,
        responder_capabilities.symbiosis_score,
        1.0 - responder_capabilities.coherence_temperature.min(1.0),
    ];
    let capability_similarity = cosine_similarity(&cap_vec_a, &cap_vec_b);

    // Reputation component.
    let reputation_component = config.beta * requester.reputation + config.gamma * responder_reputation;

    // Threat penalty.
    let threat_penalty = config.delta * threat_score;

    // Raw trust score.
    let raw_trust = config.alpha * capability_similarity + reputation_component - threat_penalty;

    // Sigmoid to [0, 1].
    let trust_score = sigmoid(raw_trust);

    // Acceptance threshold: trust > 0.5.
    let accepted = trust_score > 0.5;

    // Neighbor weight proportional to trust.
    let neighbor_weight = if accepted { trust_score } else { 0.0 };

    HandshakeResult {
        trust_score,
        capability_similarity,
        reputation_component,
        threat_penalty,
        accepted,
        neighbor_weight,
    }
}

/// Propagate Symbiotic State — Gossip-based state synchronization with VFE gradient descent.
///
/// ```text
/// Δstate_i = λ · Σ w_ij · (state_j - state_i) - ε · ∇VFE(state_i)
/// ```
///
/// # Parameters
/// - `local_state`: Current local symbiotic state.
/// - `neighbor_states`: States received from neighboring nodes.
/// - `neighbor_weights`: Weights for each neighbor (from handshake trust).
/// - `config`: USP configuration.
///
/// # Returns
/// `PropagationResult` with updated state and convergence metrics.
pub fn propagate_symbiotic_state(
    local_state: &SymbioticState,
    neighbor_states: &[SymbioticState],
    neighbor_weights: &[f64],
    config: &UspConfig,
) -> PropagationResult {
    let n = local_state.influence_shares.len();
    if n == 0 || neighbor_states.is_empty() {
        return PropagationResult {
            updated_shares: local_state.influence_shares.clone(),
            updated_vfe: local_state.vfe_values.clone(),
            delta_norm: 0.0,
            hops_consumed: 0,
            converged: true,
            neighbors_processed: 0,
        };
    }

    let mut new_shares = local_state.influence_shares.clone();
    let mut new_vfe = local_state.vfe_values.clone();
    let mut total_delta = 0.0;

    let neighbors_processed = neighbor_states.len().min(neighbor_weights.len());

    for idx in 0..neighbors_processed {
        let neighbor = &neighbor_states[idx];
        let w = neighbor_weights[idx];
        if w < 1e-15 {
            continue;
        }

        // Influence share propagation.
        let ns = neighbor.influence_shares.len().min(n);
        for i in 0..ns {
            let delta_share = config.propagation_lr * w * (neighbor.influence_shares[i] - new_shares[i]);
            new_shares[i] += delta_share;
            total_delta += delta_share * delta_share;
        }

        // VFE gradient propagation.
        let nv = neighbor.vfe_values.len().min(n);
        for i in 0..nv {
            let vfe_grad = config.vfe_gradient_weight * (neighbor.vfe_values[i] - new_vfe[i]);
            new_vfe[i] -= vfe_grad;
        }
    }

    // Normalize influence shares to sum to 1.
    let sum: f64 = new_shares.iter().sum();
    if sum > 1e-15 {
        for s in &mut new_shares {
            *s = (*s / sum).max(0.0);
        }
    }

    let delta_norm = total_delta.sqrt();
    let converged = delta_norm < 1e-8;

    PropagationResult {
        updated_shares: new_shares,
        updated_vfe: new_vfe,
        delta_norm,
        hops_consumed: 1,
        converged,
        neighbors_processed,
    }
}

/// Verify Universal Protocol Compliance — Check all USP invariants.
///
/// ```text
/// Compliance = (1 - KL(prior || safe_prior)) · (1 - Byzantine_ratio) · coherence
/// ```
///
/// # Parameters
/// - `state`: Current symbiotic state to verify.
/// - `safe_prior`: Reference safe prior distribution.
/// - `byzantine_count`: Number of detected Byzantine nodes.
/// - `total_nodes`: Total number of nodes in the mesh.
/// - `config`: USP configuration.
///
/// # Returns
/// `ComplianceResult` with compliance score and violated invariants.
pub fn verify_universal_protocol_compliance(
    state: &SymbioticState,
    safe_prior: &[f64],
    byzantine_count: usize,
    total_nodes: usize,
    config: &UspConfig,
) -> ComplianceResult {
    let mut violated_invariants = Vec::new();

    // Prior divergence.
    let prior_divergence = if !state.safe_prior.is_empty() && !safe_prior.is_empty() {
        kl_divergence(&state.safe_prior, safe_prior)
    } else {
        0.0
    };

    // Byzantine ratio.
    let byzantine_ratio = if total_nodes > 0 {
        byzantine_count as f64 / total_nodes as f64
    } else {
        0.0
    };

    // Coherence factor.
    let coherence_factor = state.coherence_score.clamp(0.0, 1.0);

    // Compliance score.
    let compliance_score = (1.0 - prior_divergence.min(1.0))
        * (1.0 - byzantine_ratio)
        * coherence_factor;

    // Check invariants.
    // Invariant 1: Influence shares sum to 1.
    let shares_sum: f64 = state.influence_shares.iter().sum();
    if (shares_sum - 1.0).abs() > 1e-6 && !state.influence_shares.is_empty() {
        violated_invariants.push("influence_shares_sum".to_string());
    }

    // Invariant 2: All influence shares non-negative.
    for (i, &s) in state.influence_shares.iter().enumerate() {
        if s < -1e-9 {
            violated_invariants.push(format!("negative_share_{}", i));
            break;
        }
    }

    // Invariant 3: Coherence in [0, 1].
    if state.coherence_score < -1e-9 || state.coherence_score > 1.0 + 1e-9 {
        violated_invariants.push("coherence_out_of_range".to_string());
    }

    // Invariant 4: Hops within TTL.
    if state.hops > config.max_hops {
        violated_invariants.push("hops_exceed_ttl".to_string());
    }

    // Invariant 5: Byzantine ratio below threshold (< 1/3 for consensus).
    if byzantine_ratio > 1.0 / 3.0 {
        violated_invariants.push("byzantine_ratio_exceeds_threshold".to_string());
    }

    let compliant = compliance_score >= config.compliance_tolerance
        && violated_invariants.is_empty();

    ComplianceResult {
        prior_divergence,
        byzantine_ratio,
        coherence_factor,
        compliance_score,
        compliant,
        violated_invariants,
    }
}

/// Run full USP pipeline — Handshake, propagate, verify compliance.
///
/// # Parameters
/// - `local_state`: Current local symbiotic state.
/// - `handshake_request`: Incoming handshake request.
/// - `responder_capabilities`: Our capabilities for handshake.
/// - `our_reputation`: Our reputation score.
/// - `threat_score`: Threat score for requester.
/// - `neighbor_states`: States from existing neighbors.
/// - `neighbor_weights`: Weights for existing neighbors.
/// - `safe_prior`: Reference safe prior.
/// - `byzantine_count`: Detected Byzantine nodes.
/// - `total_nodes`: Total mesh size.
/// - `config`: USP configuration.
///
/// # Returns
/// Tuple of (HandshakeResult, PropagationResult, ComplianceResult).
pub fn run_usp_pipeline(
    local_state: &SymbioticState,
    handshake_request: &HandshakeRequest,
    responder_capabilities: &Capability,
    our_reputation: f64,
    threat_score: f64,
    neighbor_states: &[SymbioticState],
    neighbor_weights: &[f64],
    safe_prior: &[f64],
    byzantine_count: usize,
    total_nodes: usize,
    config: &UspConfig,
) -> (HandshakeResult, PropagationResult, ComplianceResult) {
    let handshake = usp_handshake(handshake_request, responder_capabilities, our_reputation, threat_score, config);
    let propagation = propagate_symbiotic_state(local_state, neighbor_states, neighbor_weights, config);
    let compliance = verify_universal_protocol_compliance(local_state, safe_prior, byzantine_count, total_nodes, config);
    (handshake, propagation, compliance)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Config Tests ---

    #[test]
    fn test_usp_config_default() {
        let cfg = UspConfig::default();
        assert!((cfg.alpha - 0.4).abs() < 1e-9);
        assert!((cfg.beta - 0.25).abs() < 1e-9);
        assert!((cfg.gamma - 0.25).abs() < 1e-9);
        assert!((cfg.delta - 1.5).abs() < 1e-9);
        assert!((cfg.propagation_lr - 0.1).abs() < 1e-9);
        assert!((cfg.vfe_gradient_weight - 0.05).abs() < 1e-9);
        assert_eq!(cfg.max_hops, 16);
        assert!((cfg.compliance_tolerance - 0.9).abs() < 1e-9);
        assert_eq!(cfg.seed, 42);
    }

    #[test]
    fn test_usp_config_with_alpha() {
        let cfg = UspConfig::default().with_alpha(0.6);
        assert!((cfg.alpha - 0.6).abs() < 1e-9);
    }

    #[test]
    fn test_usp_config_alpha_clamped_high() {
        let cfg = UspConfig::default().with_alpha(1.5);
        assert!((cfg.alpha - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_usp_config_alpha_clamped_low() {
        let cfg = UspConfig::default().with_alpha(-0.5);
        assert!((cfg.alpha - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_usp_config_with_beta() {
        let cfg = UspConfig::default().with_beta(0.3);
        assert!((cfg.beta - 0.3).abs() < 1e-9);
    }

    #[test]
    fn test_usp_config_with_gamma() {
        let cfg = UspConfig::default().with_gamma(0.3);
        assert!((cfg.gamma - 0.3).abs() < 1e-9);
    }

    #[test]
    fn test_usp_config_with_delta() {
        let cfg = UspConfig::default().with_delta(2.0);
        assert!((cfg.delta - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_usp_config_delta_positive() {
        let cfg = UspConfig::default().with_delta(-1.0);
        assert!(cfg.delta >= 0.0);
    }

    #[test]
    fn test_usp_config_with_propagation_lr() {
        let cfg = UspConfig::default().with_propagation_lr(0.15);
        assert!((cfg.propagation_lr - 0.15).abs() < 1e-9);
    }

    #[test]
    fn test_usp_config_lr_clamped_low() {
        let cfg = UspConfig::default().with_propagation_lr(0.00001);
        assert!(cfg.propagation_lr >= 0.0001);
    }

    #[test]
    fn test_usp_config_lr_clamped_high() {
        let cfg = UspConfig::default().with_propagation_lr(2.0);
        assert!(cfg.propagation_lr <= 1.0);
    }

    #[test]
    fn test_usp_config_with_vfe_gradient_weight() {
        let cfg = UspConfig::default().with_vfe_gradient_weight(0.1);
        assert!((cfg.vfe_gradient_weight - 0.1).abs() < 1e-9);
    }

    #[test]
    fn test_usp_config_with_max_hops() {
        let cfg = UspConfig::default().with_max_hops(8);
        assert_eq!(cfg.max_hops, 8);
    }

    #[test]
    fn test_usp_config_max_hops_min() {
        let cfg = UspConfig::default().with_max_hops(0);
        assert!(cfg.max_hops >= 1);
    }

    #[test]
    fn test_usp_config_with_compliance_tolerance() {
        let cfg = UspConfig::default().with_compliance_tolerance(0.95);
        assert!((cfg.compliance_tolerance - 0.95).abs() < 1e-9);
    }

    #[test]
    fn test_usp_config_with_seed() {
        let cfg = UspConfig::default().with_seed(123);
        assert_eq!(cfg.seed, 123);
    }

    #[test]
    fn test_usp_config_fast() {
        let cfg = UspConfig::fast();
        assert!((cfg.propagation_lr - 0.2).abs() < 1e-9);
        assert_eq!(cfg.max_hops, 4);
        assert!((cfg.compliance_tolerance - 0.7).abs() < 1e-9);
    }

    #[test]
    fn test_usp_config_high_precision() {
        let cfg = UspConfig::high_precision();
        assert!((cfg.propagation_lr - 0.01).abs() < 1e-9);
        assert!((cfg.vfe_gradient_weight - 0.02).abs() < 1e-9);
        assert_eq!(cfg.max_hops, 32);
        assert!((cfg.compliance_tolerance - 0.99).abs() < 1e-9);
    }

    // --- Math Helper Tests ---

    #[test]
    fn test_sigmoid_zero() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_sigmoid_large_positive() {
        assert!(sigmoid(10.0) > 0.999);
    }

    #[test]
    fn test_sigmoid_large_negative() {
        assert!(sigmoid(-10.0) < 0.001);
    }

    #[test]
    fn test_sigmoid_range() {
        for x in [-100.0, -10.0, -1.0, 0.0, 1.0, 10.0, 100.0] {
            let s = sigmoid(x);
            assert!(s >= 0.0 && s <= 1.0, "sigmoid({}) = {}", x, s);
        }
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        assert!((cosine_similarity(&a, &a) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        assert!((cosine_similarity(&a, &b) + 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a: Vec<f64> = vec![];
        assert!((cosine_similarity(&a, &a) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim >= -1.0 && sim <= 1.0);
    }

    #[test]
    fn test_kl_divergence_identical() {
        let p = vec![0.25, 0.25, 0.25, 0.25];
        assert!((kl_divergence(&p, &p) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_kl_divergence_positive() {
        let p = vec![0.5, 0.5];
        let q = vec![0.1, 0.9];
        assert!(kl_divergence(&p, &q) > 0.0);
    }

    #[test]
    fn test_kl_divergence_asymmetric() {
        let p = vec![0.5, 0.5];
        let q = vec![0.1, 0.9];
        let kl_pq = kl_divergence(&p, &q);
        let kl_qp = kl_divergence(&q, &p);
        assert!((kl_pq - kl_qp).abs() > 1e-6);
    }

    #[test]
    fn test_shannon_entropy_uniform() {
        let dist = vec![0.25, 0.25, 0.25, 0.25];
        let h = shannon_entropy(&dist);
        assert!((h - (4f64.ln())).abs() < 1e-6);
    }

    #[test]
    fn test_shannon_entropy_deterministic() {
        let dist = vec![1.0, 0.0, 0.0];
        assert!((shannon_entropy(&dist) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_shannon_entropy_positive() {
        let dist = vec![0.3, 0.3, 0.4];
        assert!(shannon_entropy(&dist) > 0.0);
    }

    #[test]
    fn test_euclidean_norm_basic() {
        let v = vec![3.0, 4.0];
        assert!((euclidean_norm(&v) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_euclidean_norm_empty() {
        let v: Vec<f64> = vec![];
        assert!((euclidean_norm(&v) - 0.0).abs() < 1e-9);
    }

    // --- LCG Tests ---

    #[test]
    fn test_lcg_next_deterministic() {
        let mut s1: u64 = 42;
        let mut s2: u64 = 42;
        assert_eq!(lcg_next(&mut s1), lcg_next(&mut s2));
    }

    #[test]
    fn test_lcg_next_advances() {
        let mut s: u64 = 42;
        let v1 = lcg_next(&mut s);
        let v2 = lcg_next(&mut s);
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_random_uniform_range() {
        let mut s: u64 = 123;
        for _ in 0..100 {
            let r = random_uniform(&mut s);
            assert!(r >= 0.0 && r <= 1.0);
        }
    }

    // --- Capability Tests ---

    #[test]
    fn test_capability_default() {
        let cap = Capability::default();
        assert_eq!(cap.node_type, "standard");
        assert!((cap.vfe_capability - 0.5).abs() < 1e-9);
        assert!((cap.energy_efficiency - 0.5).abs() < 1e-9);
        assert!((cap.symbiosis_score - 0.5).abs() < 1e-9);
        assert!((cap.coherence_temperature - 1.0).abs() < 1e-9);
    }

    // --- Handshake Tests ---

    #[test]
    fn test_usp_handshake_identical_caps() {
        let cfg = UspConfig::default();
        let cap = Capability::default();
        let request = HandshakeRequest {
            requester_id: "A".to_string(),
            capabilities: cap.clone(),
            reputation: 0.8,
            timestamp: 1000,
        };
        let result = usp_handshake(&request, &cap, 0.8, 0.0, &cfg);
        assert!(result.capability_similarity > 0.9);
        assert!(result.accepted);
        assert!(result.trust_score > 0.5);
    }

    #[test]
    fn test_usp_handshake_high_threat() {
        let cfg = UspConfig::default();
        let cap = Capability::default();
        let request = HandshakeRequest {
            requester_id: "A".to_string(),
            capabilities: cap.clone(),
            reputation: 0.8,
            timestamp: 1000,
        };
        let result = usp_handshake(&request, &cap, 0.8, 1.0, &cfg);
        assert!(result.threat_penalty > 0.0);
        // High threat should reduce trust.
        let clean_result = usp_handshake(&request, &cap, 0.8, 0.0, &cfg);
        assert!(result.trust_score < clean_result.trust_score);
    }

    #[test]
    fn test_usp_handshake_low_reputation() {
        let cfg = UspConfig::default();
        let cap = Capability::default();
        let request = HandshakeRequest {
            requester_id: "A".to_string(),
            capabilities: cap.clone(),
            reputation: 0.0,
            timestamp: 1000,
        };
        let result = usp_handshake(&request, &cap, 0.0, 0.0, &cfg);
        assert!((result.reputation_component - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_usp_handshake_different_caps() {
        let cfg = UspConfig::default();
        let cap_a = Capability {
            vfe_capability: 1.0,
            energy_efficiency: 1.0,
            symbiosis_score: 1.0,
            coherence_temperature: 0.0,
            ..Capability::default()
        };
        let cap_b = Capability {
            vfe_capability: 0.0,
            energy_efficiency: 0.0,
            symbiosis_score: 0.0,
            coherence_temperature: 2.0,
            ..Capability::default()
        };
        let request = HandshakeRequest {
            requester_id: "A".to_string(),
            capabilities: cap_a,
            reputation: 0.5,
            timestamp: 1000,
        };
        let result = usp_handshake(&request, &cap_b, 0.5, 0.0, &cfg);
        assert!(result.capability_similarity < 0.5);
    }

    #[test]
    fn test_usp_handshake_trust_range() {
        let cfg = UspConfig::default();
        let cap = Capability::default();
        let request = HandshakeRequest {
            requester_id: "A".to_string(),
            capabilities: cap,
            reputation: 0.5,
            timestamp: 1000,
        };
        let result = usp_handshake(&request, &Capability::default(), 0.5, 0.0, &cfg);
        assert!(result.trust_score >= 0.0 && result.trust_score <= 1.0);
    }

    #[test]
    fn test_usp_handshake_accepted_when_trust_high() {
        let cfg = UspConfig::default();
        let cap = Capability::default();
        let request = HandshakeRequest {
            requester_id: "A".to_string(),
            capabilities: cap,
            reputation: 1.0,
            timestamp: 1000,
        };
        let result = usp_handshake(&request, &Capability::default(), 1.0, 0.0, &cfg);
        assert!(result.accepted);
        assert!(result.neighbor_weight > 0.0);
    }

    #[test]
    fn test_usp_handshake_rejected_when_trust_low() {
        let cfg = UspConfig::default();
        let cap = Capability::default();
        let request = HandshakeRequest {
            requester_id: "A".to_string(),
            capabilities: cap,
            reputation: 0.0,
            timestamp: 1000,
        };
        let result = usp_handshake(&request, &Capability::default(), 0.0, 2.0, &cfg);
        assert!(!result.accepted);
        assert!((result.neighbor_weight - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_handshake_result_display() {
        let result = HandshakeResult {
            trust_score: 0.75,
            capability_similarity: 0.8,
            reputation_component: 0.4,
            threat_penalty: 0.1,
            accepted: true,
            neighbor_weight: 0.75,
        };
        let display = format!("{}", result);
        assert!(display.contains("0.75"));
        assert!(display.contains("accepted: true"));
    }

    // --- Propagation Tests ---

    #[test]
    fn test_propagate_empty_local() {
        let cfg = UspConfig::default();
        let local = SymbioticState::default();
        let result = propagate_symbiotic_state(&local, &[], &[], &cfg);
        assert!(result.converged);
        assert!((result.delta_norm - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_propagate_empty_neighbors() {
        let cfg = UspConfig::default();
        let local = SymbioticState {
            influence_shares: vec![0.5, 0.5],
            vfe_values: vec![0.3, 0.3],
            ..SymbioticState::default()
        };
        let result = propagate_symbiotic_state(&local, &[], &[], &cfg);
        assert!(result.converged);
        assert_eq!(result.updated_shares, vec![0.5, 0.5]);
    }

    #[test]
    fn test_propagate_convergence_identical() {
        let cfg = UspConfig::default();
        let state = SymbioticState {
            influence_shares: vec![0.5, 0.5],
            vfe_values: vec![0.3, 0.3],
            ..SymbioticState::default()
        };
        let neighbor = state.clone();
        let result = propagate_symbiotic_state(&state, &[neighbor], &[1.0], &cfg);
        assert!(result.converged);
        assert!((result.delta_norm - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_propagate_shares_normalize() {
        let cfg = UspConfig::default();
        let local = SymbioticState {
            influence_shares: vec![0.3, 0.3],
            vfe_values: vec![0.5, 0.5],
            ..SymbioticState::default()
        };
        let neighbor = SymbioticState {
            influence_shares: vec![0.8, 0.2],
            vfe_values: vec![0.2, 0.8],
            ..SymbioticState::default()
        };
        let result = propagate_symbiotic_state(&local, &[neighbor], &[1.0], &cfg);
        let sum: f64 = result.updated_shares.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_propagate_delta_positive() {
        let cfg = UspConfig::default();
        let local = SymbioticState {
            influence_shares: vec![0.5, 0.5],
            vfe_values: vec![0.5, 0.5],
            ..SymbioticState::default()
        };
        let neighbor = SymbioticState {
            influence_shares: vec![1.0, 0.0],
            vfe_values: vec![0.0, 1.0],
            ..SymbioticState::default()
        };
        let result = propagate_symbiotic_state(&local, &[neighbor], &[1.0], &cfg);
        assert!(result.delta_norm > 0.0);
    }

    #[test]
    fn test_propagate_multiple_neighbors() {
        let cfg = UspConfig::default();
        let local = SymbioticState {
            influence_shares: vec![0.33, 0.33, 0.34],
            vfe_values: vec![0.5, 0.5, 0.5],
            ..SymbioticState::default()
        };
        let n1 = SymbioticState {
            influence_shares: vec![0.5, 0.3, 0.2],
            vfe_values: vec![0.3, 0.4, 0.3],
            ..SymbioticState::default()
        };
        let n2 = SymbioticState {
            influence_shares: vec![0.2, 0.5, 0.3],
            vfe_values: vec![0.4, 0.3, 0.3],
            ..SymbioticState::default()
        };
        let result = propagate_symbiotic_state(&local, &[n1, n2], &[0.5, 0.5], &cfg);
        assert_eq!(result.neighbors_processed, 2);
        let sum: f64 = result.updated_shares.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_propagate_zero_weight_skips() {
        let cfg = UspConfig::default();
        let local = SymbioticState {
            influence_shares: vec![0.5, 0.5],
            vfe_values: vec![0.5, 0.5],
            ..SymbioticState::default()
        };
        let neighbor = SymbioticState {
            influence_shares: vec![1.0, 0.0],
            vfe_values: vec![0.0, 1.0],
            ..SymbioticState::default()
        };
        let result = propagate_symbiotic_state(&local, &[neighbor], &[0.0], &cfg);
        assert!((result.delta_norm - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_propagation_result_display() {
        let result = PropagationResult {
            updated_shares: vec![0.5, 0.5],
            updated_vfe: vec![0.3, 0.3],
            delta_norm: 0.1,
            hops_consumed: 1,
            converged: false,
            neighbors_processed: 2,
        };
        let display = format!("{}", result);
        assert!(display.contains("0.1"));
        assert!(display.contains("converged: false"));
    }

    // --- Compliance Tests ---

    #[test]
    fn test_compliance_perfect() {
        let cfg = UspConfig::default();
        let safe_prior = vec![0.25, 0.25, 0.25, 0.25];
        let state = SymbioticState {
            influence_shares: vec![0.25, 0.25, 0.25, 0.25],
            safe_prior: safe_prior.clone(),
            coherence_score: 1.0,
            ..SymbioticState::default()
        };
        let result = verify_universal_protocol_compliance(&state, &safe_prior, 0, 10, &cfg);
        assert!(result.compliant);
        assert!((result.prior_divergence - 0.0).abs() < 1e-9);
        assert!((result.byzantine_ratio - 0.0).abs() < 1e-9);
        assert!((result.coherence_factor - 1.0).abs() < 1e-9);
        assert!(result.compliance_score >= cfg.compliance_tolerance);
    }

    #[test]
    fn test_compliance_byzantine_failure() {
        let cfg = UspConfig::default();
        let safe_prior = vec![0.25, 0.25, 0.25, 0.25];
        let state = SymbioticState {
            influence_shares: vec![0.25, 0.25, 0.25, 0.25],
            safe_prior: safe_prior.clone(),
            coherence_score: 1.0,
            ..SymbioticState::default()
        };
        // 4 out of 10 Byzantine > 1/3 threshold.
        let result = verify_universal_protocol_compliance(&state, &safe_prior, 4, 10, &cfg);
        assert!(!result.compliant);
        assert!(result.violated_invariants.contains(&"byzantine_ratio_exceeds_threshold".to_string()));
    }

    #[test]
    fn test_compliance_prior_divergence() {
        let cfg = UspConfig::default();
        let safe_prior = vec![0.5, 0.5];
        let state = SymbioticState {
            influence_shares: vec![0.5, 0.5],
            safe_prior: vec![0.9, 0.1],
            coherence_score: 1.0,
            ..SymbioticState::default()
        };
        let result = verify_universal_protocol_compliance(&state, &safe_prior, 0, 10, &cfg);
        assert!(result.prior_divergence > 0.0);
    }

    #[test]
    fn test_compliance_shares_sum_invariant() {
        let cfg = UspConfig::default();
        let safe_prior = vec![0.5, 0.5];
        let state = SymbioticState {
            influence_shares: vec![0.5, 0.6], // sums to 1.1
            safe_prior: safe_prior.clone(),
            coherence_score: 1.0,
            ..SymbioticState::default()
        };
        let result = verify_universal_protocol_compliance(&state, &safe_prior, 0, 10, &cfg);
        assert!(result.violated_invariants.contains(&"influence_shares_sum".to_string()));
    }

    #[test]
    fn test_compliance_negative_share_invariant() {
        let cfg = UspConfig::default();
        let safe_prior = vec![0.5, 0.5];
        let state = SymbioticState {
            influence_shares: vec![0.6, -0.1],
            safe_prior: safe_prior.clone(),
            coherence_score: 1.0,
            ..SymbioticState::default()
        };
        let result = verify_universal_protocol_compliance(&state, &safe_prior, 0, 10, &cfg);
        assert!(result.violated_invariants.iter().any(|s| s.starts_with("negative_share_")));
    }

    #[test]
    fn test_compliance_coherence_out_of_range() {
        let cfg = UspConfig::default();
        let safe_prior = vec![0.5, 0.5];
        let state = SymbioticState {
            influence_shares: vec![0.5, 0.5],
            safe_prior: safe_prior.clone(),
            coherence_score: 1.5,
            ..SymbioticState::default()
        };
        let result = verify_universal_protocol_compliance(&state, &safe_prior, 0, 10, &cfg);
        assert!(result.violated_invariants.contains(&"coherence_out_of_range".to_string()));
    }

    #[test]
    fn test_compliance_hops_exceed_ttl() {
        let cfg = UspConfig::fast(); // max_hops = 4
        let safe_prior = vec![0.5, 0.5];
        let state = SymbioticState {
            influence_shares: vec![0.5, 0.5],
            safe_prior: safe_prior.clone(),
            coherence_score: 1.0,
            hops: 10,
            ..SymbioticState::default()
        };
        let result = verify_universal_protocol_compliance(&state, &safe_prior, 0, 10, &cfg);
        assert!(result.violated_invariants.contains(&"hops_exceed_ttl".to_string()));
    }

    #[test]
    fn test_compliance_empty_state() {
        let cfg = UspConfig::default();
        let safe_prior: Vec<f64> = vec![];
        let state = SymbioticState::default();
        let result = verify_universal_protocol_compliance(&state, &safe_prior, 0, 0, &cfg);
        assert!((result.prior_divergence - 0.0).abs() < 1e-9);
        assert!((result.byzantine_ratio - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_compliance_score_range() {
        let cfg = UspConfig::default();
        let safe_prior = vec![0.5, 0.5];
        let state = SymbioticState {
            influence_shares: vec![0.5, 0.5],
            safe_prior: safe_prior.clone(),
            coherence_score: 0.5,
            ..SymbioticState::default()
        };
        let result = verify_universal_protocol_compliance(&state, &safe_prior, 0, 10, &cfg);
        assert!(result.compliance_score >= 0.0 && result.compliance_score <= 1.0);
    }

    #[test]
    fn test_compliance_result_display() {
        let result = ComplianceResult {
            prior_divergence: 0.1,
            byzantine_ratio: 0.05,
            coherence_factor: 0.9,
            compliance_score: 0.78,
            compliant: true,
            violated_invariants: vec![],
        };
        let display = format!("{}", result);
        assert!(display.contains("0.78"));
        assert!(display.contains("compliant: true"));
    }

    // --- Message Tests ---

    #[test]
    fn test_handshake_request_creation() {
        let req = HandshakeRequest {
            requester_id: "node-1".to_string(),
            capabilities: Capability::default(),
            reputation: 0.7,
            timestamp: 12345,
        };
        assert_eq!(req.requester_id, "node-1");
        assert!((req.reputation - 0.7).abs() < 1e-9);
    }

    #[test]
    fn test_handshake_response_creation() {
        let resp = HandshakeResponse {
            responder_id: "node-2".to_string(),
            trust_score: 0.8,
            accepted: true,
            neighbor_weight: 0.8,
            timestamp: 12346,
        };
        assert!(resp.accepted);
        assert!((resp.trust_score - 0.8).abs() < 1e-9);
    }

    #[test]
    fn test_symbiotic_state_default() {
        let state = SymbioticState::default();
        assert!(state.source_id.is_empty());
        assert!(state.influence_shares.is_empty());
        assert!(state.vfe_values.is_empty());
        assert!((state.coherence_score - 0.0).abs() < 1e-9);
        assert_eq!(state.hops, 0);
        assert_eq!(state.sequence, 0);
    }

    #[test]
    fn test_propagation_message_creation() {
        let state = SymbioticState {
            source_id: "node-1".to_string(),
            influence_shares: vec![0.5, 0.5],
            ..SymbioticState::default()
        };
        let msg = PropagationMessage {
            origin_id: "node-1".to_string(),
            state: state.clone(),
            current_hop: 0,
            timestamp: 1000,
        };
        assert_eq!(msg.origin_id, "node-1");
        assert_eq!(msg.current_hop, 0);
    }

    // --- Pipeline Tests ---

    #[test]
    fn test_run_usp_pipeline() {
        let cfg = UspConfig::default();
        let safe_prior = vec![0.25, 0.25, 0.25, 0.25];
        let local = SymbioticState {
            influence_shares: vec![0.25, 0.25, 0.25, 0.25],
            vfe_values: vec![0.5, 0.5, 0.5, 0.5],
            safe_prior: safe_prior.clone(),
            coherence_score: 0.95,
            ..SymbioticState::default()
        };
        let request = HandshakeRequest {
            requester_id: "new-node".to_string(),
            capabilities: Capability::default(),
            reputation: 0.7,
            timestamp: 1000,
        };
        let neighbor = SymbioticState {
            influence_shares: vec![0.3, 0.3, 0.2, 0.2],
            vfe_values: vec![0.4, 0.4, 0.6, 0.6],
            ..SymbioticState::default()
        };

        let (handshake, propagation, compliance) = run_usp_pipeline(
            &local,
            &request,
            &Capability::default(),
            0.7,
            0.0,
            &[neighbor],
            &[1.0],
            &safe_prior,
            0,
            10,
            &cfg,
        );

        assert!(handshake.trust_score >= 0.0 && handshake.trust_score <= 1.0);
        assert_eq!(propagation.neighbors_processed, 1);
        assert!(compliance.compliance_score >= 0.0);
    }

    #[test]
    fn test_run_usp_pipeline_empty() {
        let cfg = UspConfig::default();
        let local = SymbioticState::default();
        let request = HandshakeRequest {
            requester_id: "new".to_string(),
            capabilities: Capability::default(),
            reputation: 0.5,
            timestamp: 1000,
        };
        let safe_prior: Vec<f64> = vec![];

        let (handshake, propagation, compliance) = run_usp_pipeline(
            &local,
            &request,
            &Capability::default(),
            0.5,
            0.0,
            &[],
            &[],
            &safe_prior,
            0,
            0,
            &cfg,
        );

        assert!(handshake.trust_score >= 0.0);
        assert!(propagation.converged);
    }

    #[test]
    fn test_run_usp_pipeline_with_threat() {
        let cfg = UspConfig::default();
        let safe_prior = vec![0.5, 0.5];
        let local = SymbioticState {
            influence_shares: vec![0.5, 0.5],
            safe_prior: safe_prior.clone(),
            coherence_score: 1.0,
            ..SymbioticState::default()
        };
        let request = HandshakeRequest {
            requester_id: "threat".to_string(),
            capabilities: Capability::default(),
            reputation: 0.0,
            timestamp: 1000,
        };

        let (handshake, _, _) = run_usp_pipeline(
            &local,
            &request,
            &Capability::default(),
            0.5,
            1.0, // High threat
            &[],
            &[],
            &safe_prior,
            0,
            10,
            &cfg,
        );

        assert!(handshake.threat_penalty > 0.0);
    }

    // --- Integration / Edge Cases ---

    #[test]
    fn test_handshake_deterministic() {
        let cfg = UspConfig::default();
        let cap = Capability::default();
        let request = HandshakeRequest {
            requester_id: "A".to_string(),
            capabilities: cap.clone(),
            reputation: 0.7,
            timestamp: 1000,
        };
        let r1 = usp_handshake(&request, &cap, 0.7, 0.0, &cfg);
        let r2 = usp_handshake(&request, &cap, 0.7, 0.0, &cfg);
        assert!((r1.trust_score - r2.trust_score).abs() < 1e-12);
    }

    #[test]
    fn test_propagation_vfe_decreases() {
        let cfg = UspConfig::default();
        let local = SymbioticState {
            influence_shares: vec![0.5, 0.5],
            vfe_values: vec![1.0, 1.0],
            ..SymbioticState::default()
        };
        let neighbor = SymbioticState {
            influence_shares: vec![0.5, 0.5],
            vfe_values: vec![0.0, 0.0],
            ..SymbioticState::default()
        };
        let result = propagate_symbiotic_state(&local, &[neighbor], &[1.0], &cfg);
        // VFE should decrease toward neighbor's lower values.
        assert!(result.updated_vfe[0] < 1.0);
        assert!(result.updated_vfe[1] < 1.0);
    }

    #[test]
    fn test_compliance_bounded_byzantine() {
        let cfg = UspConfig::default();
        let safe_prior = vec![0.5, 0.5];
        let state = SymbioticState {
            influence_shares: vec![0.5, 0.5],
            safe_prior: safe_prior.clone(),
            coherence_score: 1.0,
            ..SymbioticState::default()
        };
        // 0 Byzantine out of 0 nodes = 0 ratio.
        let r0 = verify_universal_protocol_compliance(&state, &safe_prior, 0, 0, &cfg);
        assert!((r0.byzantine_ratio - 0.0).abs() < 1e-9);

        // 1 Byzantine out of 3 = 0.333... which is at threshold.
        let r1 = verify_universal_protocol_compliance(&state, &safe_prior, 1, 3, &cfg);
        assert!((r1.byzantine_ratio - 1.0/3.0).abs() < 1e-9);
    }

    #[test]
    fn test_full_usp_workflow() {
        let cfg = UspConfig::default();
        let safe_prior = vec![0.25, 0.25, 0.25, 0.25];

        // Step 1: Handshake
        let cap = Capability {
            vfe_capability: 0.8,
            energy_efficiency: 0.9,
            symbiosis_score: 0.85,
            coherence_temperature: 0.5,
            ..Capability::default()
        };
        let request = HandshakeRequest {
            requester_id: "node-A".to_string(),
            capabilities: cap.clone(),
            reputation: 0.85,
            timestamp: 1000,
        };
        let handshake = usp_handshake(&request, &cap, 0.85, 0.05, &cfg);
        assert!(handshake.accepted, "Handshake should be accepted for good node");

        // Step 2: Build local state
        let local = SymbioticState {
            source_id: "node-B".to_string(),
            influence_shares: vec![0.25, 0.25, 0.25, 0.25],
            vfe_values: vec![0.5, 0.5, 0.5, 0.5],
            safe_prior: safe_prior.clone(),
            coherence_score: 0.95,
            hops: 0,
            sequence: 1,
            ..SymbioticState::default()
        };

        // Step 3: Propagate
        let neighbor = SymbioticState {
            source_id: "node-A".to_string(),
            influence_shares: vec![0.3, 0.3, 0.2, 0.2],
            vfe_values: vec![0.4, 0.4, 0.6, 0.6],
            coherence_score: 0.9,
            hops: 1,
            sequence: 1,
            ..SymbioticState::default()
        };
        let propagation = propagate_symbiotic_state(&local, &[neighbor], &[handshake.neighbor_weight], &cfg);
        assert_eq!(propagation.neighbors_processed, 1);
        let sum: f64 = propagation.updated_shares.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6, "Shares must sum to 1");

        // Step 4: Verify compliance
        let compliance = verify_universal_protocol_compliance(&local, &safe_prior, 0, 10, &cfg);
        assert!(compliance.compliant, "Should be compliant with good state");

        // All steps succeeded.
        assert!(handshake.trust_score > 0.5);
        assert!(propagation.delta_norm >= 0.0);
        assert!(compliance.compliance_score >= cfg.compliance_tolerance);
    }
}
