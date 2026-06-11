//! Eternal Noospheric Governance — Self-Sustaining Governance Without Central Authority.
//!
//! **Sprint 134 PASO C:** Eternal Noospheric Governance.
//!
//! Implements mechanisms for self-sustaining noospheric governance without central
//! authority or economy, based on thermodynamic fitness + eternal coherence.
//! Governance rules emerge from category theory colimits over the planetary mesh.
//!
//! **Thermodynamic Voting:**
//! ```text
//! weight_i = PoUS_fitness_i · coherence_i · exp(-VFE_i / T)
//! decision = argmax_a Σ_i weight_i · preference_i(a)
//! ```
//!
//! **Emergent Eternal Rules (Category Theory):**
//! ```text
//! Rule_∞ = colimit({Rule_i, f_ij}) where f_ij = coherence(i,j) · trust(i,j)
//! Rule_∞ = argmin_R Σ_i w_i · ||R - Rule_i||² + λ · Σ_{i,j} ||f_ij(Rule_i) - R||²
//! ```
//!
//! **Eternal Governance Loop:**
//! ```text
//! For each epoch:
//!   1. Collect proposals from active nodes
//!   2. Compute thermodynamic weights
//!   3. Execute thermodynamic voting
//!   4. Update emergent rules via colimit
//!   5. Verify coherence maintenance
//!   6. Apply governance decisions to kernel state
//! ```

use serde::{Deserialize, Serialize};

// ─── Eternal Governance Configuration ────────────────────────────────────────

/// Configuration for Eternal Noospheric Governance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EternalGovernanceConfig {
    /// Temperature for thermodynamic voting (T in exp(-VFE/T)).
    pub voting_temperature: f64,
    /// Colimit regularization weight (λ in eternal rules).
    pub colimit_lambda: f64,
    /// Minimum fitness to participate in governance.
    pub min_governance_fitness: f64,
    /// Minimum coherence to vote.
    pub min_voting_coherence: f64,
    /// Proposal quorum fraction.
    pub quorum_fraction: f64,
    /// Approval threshold for proposals.
    pub approval_threshold: f64,
    /// Maximum proposals per epoch.
    pub max_proposals: usize,
    /// Governance epochs per cycle.
    pub epochs_per_cycle: usize,
    /// Convergence tolerance for rule emergence.
    pub convergence_tolerance: f64,
    /// Random seed for reproducibility.
    pub seed: u64,
}

impl Default for EternalGovernanceConfig {
    fn default() -> Self {
        Self {
            voting_temperature: 1.0,
            colimit_lambda: 0.5,
            min_governance_fitness: 0.1,
            min_voting_coherence: 0.5,
            quorum_fraction: 0.5,
            approval_threshold: 0.6,
            max_proposals: 100,
            epochs_per_cycle: 10,
            convergence_tolerance: 1e-8,
            seed: 42,
        }
    }
}

impl EternalGovernanceConfig {
    /// Create config for fast testing.
    pub fn fast() -> Self {
        Self {
            voting_temperature: 2.0,
            colimit_lambda: 0.3,
            min_governance_fitness: 0.05,
            min_voting_coherence: 0.3,
            quorum_fraction: 0.3,
            approval_threshold: 0.5,
            max_proposals: 20,
            epochs_per_cycle: 3,
            convergence_tolerance: 1e-4,
            seed: 42,
        }
    }

    /// Create config for planetary eternal governance.
    pub fn planetary_eternal() -> Self {
        Self {
            voting_temperature: 0.5,
            colimit_lambda: 1.0,
            min_governance_fitness: 0.3,
            min_voting_coherence: 0.8,
            quorum_fraction: 0.6,
            approval_threshold: 0.75,
            max_proposals: 1000,
            epochs_per_cycle: 50,
            convergence_tolerance: 1e-12,
            seed: 42,
        }
    }

    /// Set voting temperature.
    pub fn with_voting_temperature(mut self, temp: f64) -> Self {
        self.voting_temperature = temp.max(0.01);
        self
    }

    /// Set colimit lambda.
    pub fn with_colimit_lambda(mut self, lambda: f64) -> Self {
        self.colimit_lambda = lambda.clamp(0.0, 1.0);
        self
    }

    /// Set quorum fraction.
    pub fn with_quorum_fraction(mut self, fraction: f64) -> Self {
        self.quorum_fraction = fraction.clamp(0.0, 1.0);
        self
    }

    /// Set random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }
}

// ─── Governance Node ─────────────────────────────────────────────────────────

/// A node participating in eternal governance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceNode {
    /// Node identifier.
    pub node_id: u64,
    /// PoUS fitness score.
    pub fitness: f64,
    /// Coherence score.
    pub coherence: f64,
    /// Variational Free Energy.
    pub vfe: f64,
    /// Current rule vector.
    pub rule_vector: Vec<f64>,
    /// Active flag.
    pub active: bool,
}

impl GovernanceNode {
    /// Create a new governance node.
    pub fn new(node_id: u64, fitness: f64, coherence: f64, vfe: f64, rule_dim: usize) -> Self {
        let rule_vector = vec![1.0 / rule_dim as f64; rule_dim];
        Self {
            node_id,
            fitness: fitness.clamp(0.0, 1.0),
            coherence: coherence.clamp(0.0, 1.0),
            vfe: vfe.max(0.0),
            rule_vector,
            active: true,
        }
    }

    /// Compute thermodynamic voting weight.
    ///
    /// ```text
    /// weight = fitness · coherence · exp(-VFE / T)
    /// ```
    pub fn voting_weight(&self, temperature: f64) -> f64 {
        self.fitness * self.coherence * (-self.vfe / temperature).exp()
    }

    /// Check if node qualifies for governance.
    pub fn can_govern(&self, config: &EternalGovernanceConfig) -> bool {
        self.active
            && self.fitness >= config.min_governance_fitness
            && self.coherence >= config.min_voting_coherence
    }
}

// ─── Governance Proposal ─────────────────────────────────────────────────────

/// A proposal in eternal governance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EternalProposal {
    /// Proposal identifier.
    pub proposal_id: u64,
    /// Proposer node ID.
    pub proposer_id: u64,
    /// Proposed rule change (delta to apply).
    pub rule_delta: Vec<f64>,
    /// Total weight in favor.
    pub weight_for: f64,
    /// Total weight against.
    pub weight_against: f64,
    /// Total weight of voters.
    pub total_voter_weight: f64,
    /// Approved flag.
    pub approved: bool,
    /// Executed flag.
    pub executed: bool,
}

impl EternalProposal {
    /// Create a new proposal.
    pub fn new(proposal_id: u64, proposer_id: u64, rule_delta: Vec<f64>) -> Self {
        Self {
            proposal_id,
            proposer_id,
            rule_delta,
            weight_for: 0.0,
            weight_against: 0.0,
            total_voter_weight: 0.0,
            approved: false,
            executed: false,
        }
    }

    /// Cast a weighted vote.
    pub fn vote(&mut self, weight: f64, in_favor: bool) {
        self.total_voter_weight += weight;
        if in_favor {
            self.weight_for += weight;
        } else {
            self.weight_against += weight;
        }
    }

    /// Compute approval ratio.
    pub fn approval_ratio(&self) -> f64 {
        if self.total_voter_weight < 1e-15 {
            return 0.0;
        }
        self.weight_for / self.total_voter_weight
    }

    /// Finalize proposal based on config thresholds.
    pub fn finalize(
        &mut self,
        quorum_fraction: f64,
        approval_threshold: f64,
        total_eligible_weight: f64,
    ) {
        let quorum_met = self.total_voter_weight
            >= quorum_fraction * total_eligible_weight;
        let approved = self.approval_ratio() >= approval_threshold;
        self.approved = quorum_met && approved;
    }
}

// ─── Eternal Governance Result ───────────────────────────────────────────────

/// Result of eternal governance cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EternalGovernanceResult {
    /// Emergent eternal rule vector.
    pub eternal_rule: Vec<f64>,
    /// Rule convergence score (lower = more converged).
    pub rule_convergence: f64,
    /// Proposals processed.
    pub proposals_processed: usize,
    /// Proposals approved.
    pub proposals_approved: usize,
    /// Total governance weight参与.
    pub total_governance_weight: f64,
    /// Active governance nodes.
    pub active_governors: usize,
    /// Coherence maintained flag.
    pub coherence_maintained: bool,
    /// Governance epochs executed.
    pub epochs: usize,
    /// Rule trajectory (one per epoch).
    pub rule_trajectory: Vec<Vec<f64>>,
}

impl EternalGovernanceResult {
    /// Generate a summary string.
    pub fn summary(&self) -> String {
        format!(
            "EternalGov: rule_dim={}, convergence={:.8}, proposals={}/{} approved={}, governors={}, coherence={}",
            self.eternal_rule.len(),
            self.rule_convergence,
            self.proposals_approved,
            self.proposals_processed,
            self.proposals_approved,
            self.active_governors,
            self.coherence_maintained,
        )
    }
}

impl std::fmt::Display for EternalGovernanceResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

// ─── LCG Random (deterministic) ──────────────────────────────────────────────

fn lcg_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

fn random_uniform(state: &mut u64) -> f64 {
    let val = lcg_next(state);
    (val >> 11) as f64 / (1u64 << 53) as f64
}

// ─── Thermodynamic Voting ────────────────────────────────────────────────────

/// Compute thermodynamic voting weights for all eligible nodes.
pub fn compute_thermodynamic_weights(
    nodes: &[GovernanceNode],
    config: &EternalGovernanceConfig,
) -> Vec<(u64, f64)> {
    nodes
        .iter()
        .filter(|n| n.can_govern(config))
        .map(|n| (n.node_id, n.voting_weight(config.voting_temperature)))
        .collect()
}

/// Execute thermodynamic voting on a set of proposals.
pub fn execute_thermodynamic_voting(
    proposals: &mut [EternalProposal],
    nodes: &[GovernanceNode],
    config: &EternalGovernanceConfig,
    rng_state: &mut u64,
) -> (usize, usize) {
    let weights: Vec<(u64, f64)> = compute_thermodynamic_weights(nodes, config);
    let total_weight: f64 = weights.iter().map(|(_, w)| w).sum();

    let mut approved = 0;
    let mut processed = 0;

    for proposal in proposals.iter_mut().take(config.max_proposals) {
        // Each eligible node votes based on fitness alignment
        for &(node_id, weight) in &weights {
            // Vote based on proposal alignment with node's rule vector
            let alignment: f64 = proposal
                .rule_delta
                .iter()
                .zip(
                    nodes.iter()
                        .find(|n| n.node_id == node_id)
                        .map(|n| n.rule_vector.as_slice())
                        .unwrap_or(&[]),
                )
                .map(|(d, r)| d * r)
                .sum();
            let in_favor = alignment > 0.0 || random_uniform(rng_state) > 0.5;
            proposal.vote(weight, in_favor);
        }

        // Finalize proposal
        proposal.finalize(
            config.quorum_fraction,
            config.approval_threshold,
            total_weight,
        );

        if proposal.approved {
            approved += 1;
        }
        processed += 1;
    }

    (processed, approved)
}

// ─── Emergent Eternal Rules (Category Theory Colimit) ────────────────────────

/// Compute emergent eternal rules via colimit aggregation.
///
/// ```text
/// Rule_∞ = argmin_R Σ_i w_i · ||R - Rule_i||² + λ · Σ_{i,j} ||f_ij(Rule_i) - R||²
/// ```
///
/// Simplified: weighted average with coherence regularization.
pub fn compute_eternal_rule_colimit(
    nodes: &[GovernanceNode],
    config: &EternalGovernanceConfig,
) -> Vec<f64> {
    let eligible: Vec<&GovernanceNode> =
        nodes.iter().filter(|n| n.can_govern(config)).collect();

    if eligible.is_empty() {
        return vec![1.0]; // Default rule
    }

    let rule_dim = eligible[0].rule_vector.len();
    let total_weight: f64 = eligible
        .iter()
        .map(|n| n.voting_weight(config.voting_temperature))
        .sum();

    if total_weight < 1e-15 {
        return vec![1.0 / rule_dim as f64; rule_dim];
    }

    // Compute weighted average rule
    let mut rule = vec![0.0; rule_dim];
    for node in &eligible {
        let weight = node.voting_weight(config.voting_temperature);
        for (r, &rule_val) in rule.iter_mut().zip(&node.rule_vector) {
            *r += weight * rule_val;
        }
    }
    for r in &mut rule {
        *r /= total_weight;
    }

    // Apply colimit regularization (smooth toward uniform)
    let uniform = 1.0 / rule_dim as f64;
    for r in &mut rule {
        *r = (1.0 - config.colimit_lambda) * *r + config.colimit_lambda * uniform;
    }

    // Normalize
    let sum: f64 = rule.iter().sum();
    if sum > 1e-15 {
        for r in &mut rule {
            *r /= sum;
        }
    }

    rule
}

/// Compute rule convergence score.
///
/// Measures how close all nodes' rules are to the eternal rule.
pub fn compute_rule_convergence(nodes: &[GovernanceNode], eternal_rule: &[f64]) -> f64 {
    if nodes.is_empty() || eternal_rule.is_empty() {
        return 0.0;
    }
    let mut total_distance = 0.0;
    for node in nodes {
        if node.rule_vector.len() == eternal_rule.len() {
            let distance: f64 = node
                .rule_vector
                .iter()
                .zip(eternal_rule)
                .map(|(a, b)| (a - b).powi(2))
                .sum();
            total_distance += distance;
        }
    }
    -(total_distance / nodes.len() as f64).sqrt() // Negative: lower distance = higher convergence
}

// ─── Eternal Governance Loop ─────────────────────────────────────────────────

/// Run the Eternal Governance Loop.
///
/// Self-sustaining governance without central authority, based on
/// thermodynamic fitness + eternal coherence + category theory colimits.
///
/// **Algorithm:**
/// 1. For each epoch:
///    a. Identify eligible governance nodes
///    b. Generate proposals from node rule differences
///    c. Execute thermodynamic voting
///    d. Apply approved proposals to node rules
///    e. Compute emergent eternal rule via colimit
///    f. Check convergence (rule distance < tolerance)
/// 2. Return EternalGovernanceResult with full certification
pub fn eternal_governance_loop(
    nodes: &mut [GovernanceNode],
    config: &EternalGovernanceConfig,
) -> EternalGovernanceResult {
    let mut rng_state = config.seed;
    let _rule_dim = nodes.first().map(|n| n.rule_vector.len()).unwrap_or(1);

    let mut rule_trajectory = Vec::new();
    let mut total_proposals_processed = 0;
    let mut total_proposals_approved = 0;
    let mut total_governance_weight = 0.0;

    for epoch in 0..config.epochs_per_cycle {
        // Compute current eternal rule
        let current_rule = compute_eternal_rule_colimit(nodes, config);
        rule_trajectory.push(current_rule.clone());

        // Generate proposals from rule differences
        let mut proposals = Vec::new();
        let eligible: Vec<&GovernanceNode> =
            nodes.iter().filter(|n| n.can_govern(config)).collect();

        for node in &eligible {
            // Proposal: move toward eternal rule
            let delta: Vec<f64> = current_rule
                .iter()
                .zip(&node.rule_vector)
                .map(|(c, n)| (c - n) * 0.1)
                .collect();

            if delta.iter().map(|d| d.abs()).sum::<f64>() > 1e-10 {
                proposals.push(EternalProposal::new(
                    epoch as u64 * 1000 + node.node_id,
                    node.node_id,
                    delta,
                ));
            }
        }

        // Limit proposals
        proposals.truncate(config.max_proposals);

        // Execute thermodynamic voting
        let (processed, approved) = execute_thermodynamic_voting(
            &mut proposals,
            nodes,
            config,
            &mut rng_state,
        );
        total_proposals_processed += processed;
        total_proposals_approved += approved;

        // Compute total governance weight
        let weights: Vec<f64> = eligible
            .iter()
            .map(|n| n.voting_weight(config.voting_temperature))
            .collect();
        total_governance_weight += weights.iter().sum::<f64>();

        // Apply approved proposals
        for proposal in proposals.iter().filter(|p| p.approved) {
            if let Some(node) = nodes.iter_mut().find(|n| n.node_id == proposal.proposer_id) {
                for (r, d) in node.rule_vector.iter_mut().zip(&proposal.rule_delta) {
                    *r = (*r + d).clamp(0.0, 1.0);
                }
                // Normalize
                let sum: f64 = node.rule_vector.iter().sum();
                if sum > 1e-15 {
                    for r in node.rule_vector.iter_mut() {
                        *r /= sum;
                    }
                }
            }
        }

        // Check convergence
        if epoch > 0 && rule_trajectory.len() >= 2 {
            let prev = rule_trajectory[rule_trajectory.len() - 2].clone();
            let curr = rule_trajectory.last().unwrap();
            let delta: f64 = prev
                .iter()
                .zip(curr)
                .map(|(a, b)| (a - b).abs())
                .sum();
            if delta < config.convergence_tolerance {
                break; // Converged
            }
        }
    }

    // Compute final eternal rule
    let eternal_rule = compute_eternal_rule_colimit(nodes, config);
    let convergence = compute_rule_convergence(nodes, &eternal_rule);

    // Check coherence maintenance
    let eligible: Vec<&GovernanceNode> =
        nodes.iter().filter(|n| n.can_govern(config)).collect();
    let coherence_maintained = eligible.iter().all(|n| n.coherence >= config.min_voting_coherence);

    EternalGovernanceResult {
        eternal_rule,
        rule_convergence: convergence,
        proposals_processed: total_proposals_processed,
        proposals_approved: total_proposals_approved,
        total_governance_weight,
        active_governors: eligible.len(),
        coherence_maintained,
        epochs: rule_trajectory.len(),
        rule_trajectory,
    }
}

/// Generate emergent eternal rules from governance result.
pub fn emergent_eternal_rules(result: &EternalGovernanceResult) -> Vec<f64> {
    result.eternal_rule.clone()
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── EternalGovernanceConfig Tests ────────────────────────────────────

    #[test]
    fn test_eternal_governance_config_default() {
        let config = EternalGovernanceConfig::default();
        assert_eq!(config.voting_temperature, 1.0);
        assert_eq!(config.colimit_lambda, 0.5);
        assert_eq!(config.quorum_fraction, 0.5);
        assert_eq!(config.approval_threshold, 0.6);
    }

    #[test]
    fn test_eternal_governance_config_fast() {
        let config = EternalGovernanceConfig::fast();
        assert_eq!(config.epochs_per_cycle, 3);
        assert_eq!(config.max_proposals, 20);
    }

    #[test]
    fn test_eternal_governance_config_planetary_eternal() {
        let config = EternalGovernanceConfig::planetary_eternal();
        assert_eq!(config.min_voting_coherence, 0.8);
        assert_eq!(config.epochs_per_cycle, 50);
    }

    #[test]
    fn test_eternal_governance_config_with_temperature() {
        let config = EternalGovernanceConfig::default().with_voting_temperature(2.0);
        assert_eq!(config.voting_temperature, 2.0);
    }

    #[test]
    fn test_eternal_governance_config_with_lambda() {
        let config = EternalGovernanceConfig::default().with_colimit_lambda(0.8);
        assert_eq!(config.colimit_lambda, 0.8);
    }

    #[test]
    fn test_eternal_governance_config_lambda_clamped() {
        let config = EternalGovernanceConfig::default().with_colimit_lambda(1.5);
        assert_eq!(config.colimit_lambda, 1.0);
    }

    #[test]
    fn test_eternal_governance_config_with_quorum() {
        let config = EternalGovernanceConfig::default().with_quorum_fraction(0.7);
        assert_eq!(config.quorum_fraction, 0.7);
    }

    #[test]
    fn test_eternal_governance_config_with_seed() {
        let config = EternalGovernanceConfig::default().with_seed(123);
        assert_eq!(config.seed, 123);
    }

    // ─── GovernanceNode Tests ─────────────────────────────────────────────

    #[test]
    fn test_governance_node_new() {
        let node = GovernanceNode::new(1, 0.8, 0.9, 0.1, 5);
        assert_eq!(node.node_id, 1);
        assert_eq!(node.rule_vector.len(), 5);
        assert!(node.active);
    }

    #[test]
    fn test_governance_node_voting_weight() {
        let node = GovernanceNode::new(1, 0.8, 0.9, 0.1, 5);
        let weight = node.voting_weight(1.0);
        assert!(weight > 0.0);
        assert!(weight < 1.0);
    }

    #[test]
    fn test_governance_node_voting_weight_high_vfe() {
        let node = GovernanceNode::new(1, 0.8, 0.9, 10.0, 5);
        let weight = node.voting_weight(1.0);
        // High VFE -> low weight due to exp(-VFE/T)
        assert!(weight < 0.001);
    }

    #[test]
    fn test_governance_node_can_govern_eligible() {
        let config = EternalGovernanceConfig::default();
        let node = GovernanceNode::new(1, 0.5, 0.6, 0.1, 5);
        assert!(node.can_govern(&config));
    }

    #[test]
    fn test_governance_node_can_govern_low_fitness() {
        let config = EternalGovernanceConfig::default();
        let node = GovernanceNode::new(1, 0.05, 0.6, 0.1, 5);
        assert!(!node.can_govern(&config));
    }

    #[test]
    fn test_governance_node_can_govern_inactive() {
        let mut node = GovernanceNode::new(1, 0.5, 0.6, 0.1, 5);
        node.active = false;
        let config = EternalGovernanceConfig::default();
        assert!(!node.can_govern(&config));
    }

    // ─── EternalProposal Tests ────────────────────────────────────────────

    #[test]
    fn test_eternal_proposal_new() {
        let proposal = EternalProposal::new(1, 42, vec![0.1, -0.1, 0.0]);
        assert_eq!(proposal.proposal_id, 1);
        assert_eq!(proposal.proposer_id, 42);
        assert!(!proposal.approved);
    }

    #[test]
    fn test_eternal_proposal_vote_for() {
        let mut proposal = EternalProposal::new(1, 42, vec![0.1]);
        proposal.vote(0.5, true);
        assert_eq!(proposal.weight_for, 0.5);
        assert_eq!(proposal.total_voter_weight, 0.5);
    }

    #[test]
    fn test_eternal_proposal_vote_against() {
        let mut proposal = EternalProposal::new(1, 42, vec![0.1]);
        proposal.vote(0.5, false);
        assert_eq!(proposal.weight_against, 0.5);
    }

    #[test]
    fn test_eternal_proposal_approval_ratio() {
        let mut proposal = EternalProposal::new(1, 42, vec![0.1]);
        proposal.vote(0.7, true);
        proposal.vote(0.3, false);
        assert!((proposal.approval_ratio() - 0.7).abs() < 1e-10);
    }

    #[test]
    fn test_eternal_proposal_approval_ratio_empty() {
        let proposal = EternalProposal::new(1, 42, vec![0.1]);
        assert_eq!(proposal.approval_ratio(), 0.0);
    }

    #[test]
    fn test_eternal_proposal_finalize_approved() {
        let mut proposal = EternalProposal::new(1, 42, vec![0.1]);
        proposal.vote(0.8, true);
        proposal.vote(0.1, false);
        proposal.finalize(0.5, 0.6, 1.0);
        assert!(proposal.approved);
    }

    #[test]
    fn test_eternal_proposal_finalize_quorum_not_met() {
        let mut proposal = EternalProposal::new(1, 42, vec![0.1]);
        proposal.vote(0.1, true);
        proposal.finalize(0.5, 0.6, 1.0);
        assert!(!proposal.approved);
    }

    #[test]
    fn test_eternal_proposal_finalize_threshold_not_met() {
        let mut proposal = EternalProposal::new(1, 42, vec![0.1]);
        proposal.vote(0.4, true);
        proposal.vote(0.4, false);
        proposal.finalize(0.5, 0.6, 1.0);
        assert!(!proposal.approved);
    }

    // ─── Thermodynamic Voting Tests ───────────────────────────────────────

    #[test]
    fn test_compute_thermodynamic_weights() {
        let nodes = vec![
            GovernanceNode::new(1, 0.8, 0.9, 0.1, 3),
            GovernanceNode::new(2, 0.3, 0.4, 0.5, 3),
        ];
        let config = EternalGovernanceConfig::default();
        let weights = compute_thermodynamic_weights(&nodes, &config);
        assert_eq!(weights.len(), 1); // Only node 1 qualifies
    }

    #[test]
    fn test_compute_thermodynamic_weights_empty() {
        let nodes: Vec<GovernanceNode> = vec![];
        let config = EternalGovernanceConfig::default();
        let weights = compute_thermodynamic_weights(&nodes, &config);
        assert!(weights.is_empty());
    }

    #[test]
    fn test_execute_thermodynamic_voting() {
        let nodes = vec![
            GovernanceNode::new(1, 0.8, 0.9, 0.1, 3),
            GovernanceNode::new(2, 0.7, 0.8, 0.2, 3),
        ];
        let mut proposals = vec![EternalProposal::new(1, 1, vec![0.1, -0.1, 0.0])];
        let config = EternalGovernanceConfig::fast();
        let mut rng = 42u64;
        let (processed, _approved) =
            execute_thermodynamic_voting(&mut proposals, &nodes, &config, &mut rng);
        assert_eq!(processed, 1);
    }

    // ─── Eternal Rule Colimit Tests ───────────────────────────────────────

    #[test]
    fn test_compute_eternal_rule_colimit_uniform() {
        let nodes = vec![
            GovernanceNode::new(1, 0.8, 0.9, 0.1, 3),
            GovernanceNode::new(2, 0.8, 0.9, 0.1, 3),
        ];
        let config = EternalGovernanceConfig::default();
        let rule = compute_eternal_rule_colimit(&nodes, &config);
        assert_eq!(rule.len(), 3);
        let sum: f64 = rule.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_eternal_rule_colimit_empty() {
        let nodes: Vec<GovernanceNode> = vec![];
        let config = EternalGovernanceConfig::default();
        let rule = compute_eternal_rule_colimit(&nodes, &config);
        assert_eq!(rule, vec![1.0]);
    }

    #[test]
    fn test_compute_eternal_rule_colimit_single() {
        let nodes = vec![GovernanceNode::new(1, 0.8, 0.9, 0.1, 3)];
        let config = EternalGovernanceConfig::default();
        let rule = compute_eternal_rule_colimit(&nodes, &config);
        assert_eq!(rule.len(), 3);
    }

    #[test]
    fn test_compute_eternal_rule_colimit_normalized() {
        let nodes = vec![
            GovernanceNode::new(1, 0.8, 0.9, 0.1, 5),
            GovernanceNode::new(2, 0.6, 0.7, 0.3, 5),
        ];
        let config = EternalGovernanceConfig::default();
        let rule = compute_eternal_rule_colimit(&nodes, &config);
        let sum: f64 = rule.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    // ─── Rule Convergence Tests ───────────────────────────────────────────

    #[test]
    fn test_compute_rule_convergence_identical() {
        let rule = vec![0.33, 0.33, 0.34];
        let mut node = GovernanceNode::new(1, 0.8, 0.9, 0.1, 3);
        node.rule_vector = rule.clone();
        let convergence = compute_rule_convergence(&[node], &rule);
        assert!((convergence).abs() < 1e-10);
    }

    #[test]
    fn test_compute_rule_convergence_different() {
        let rule = vec![1.0, 0.0, 0.0];
        let mut node = GovernanceNode::new(1, 0.8, 0.9, 0.1, 3);
        node.rule_vector = vec![0.0, 1.0, 0.0];
        let convergence = compute_rule_convergence(&[node], &rule);
        assert!(convergence < -0.5);
    }

    #[test]
    fn test_compute_rule_convergence_empty() {
        let nodes: Vec<GovernanceNode> = vec![];
        let rule: Vec<f64> = vec![];
        let convergence = compute_rule_convergence(&nodes, &rule);
        assert_eq!(convergence, 0.0);
    }

    // ─── Full Governance Loop Tests ───────────────────────────────────────

    #[test]
    fn test_eternal_governance_loop_basic() {
        let mut nodes = vec![
            GovernanceNode::new(1, 0.8, 0.9, 0.1, 3),
            GovernanceNode::new(2, 0.7, 0.8, 0.2, 3),
            GovernanceNode::new(3, 0.6, 0.7, 0.3, 3),
        ];
        let config = EternalGovernanceConfig::fast();
        let result = eternal_governance_loop(&mut nodes, &config);
        assert_eq!(result.eternal_rule.len(), 3);
        assert!(result.epochs > 0);
        assert!(result.rule_trajectory.len() > 0);
    }

    #[test]
    fn test_eternal_governance_loop_empty() {
        let mut nodes: Vec<GovernanceNode> = vec![];
        let config = EternalGovernanceConfig::fast();
        let result = eternal_governance_loop(&mut nodes, &config);
        assert_eq!(result.active_governors, 0);
    }

    #[test]
    fn test_eternal_governance_loop_deterministic() {
        let mut nodes1 = vec![
            GovernanceNode::new(1, 0.8, 0.9, 0.1, 3),
            GovernanceNode::new(2, 0.7, 0.8, 0.2, 3),
        ];
        let mut nodes2 = nodes1.clone();
        let config = EternalGovernanceConfig::fast();
        let r1 = eternal_governance_loop(&mut nodes1, &config);
        let r2 = eternal_governance_loop(&mut nodes2, &config);
        assert_eq!(r1.eternal_rule, r2.eternal_rule);
    }

    #[test]
    fn test_eternal_governance_loop_rule_normalized() {
        let mut nodes = vec![
            GovernanceNode::new(1, 0.8, 0.9, 0.1, 5),
            GovernanceNode::new(2, 0.7, 0.8, 0.2, 5),
        ];
        let config = EternalGovernanceConfig::fast();
        let result = eternal_governance_loop(&mut nodes, &config);
        let sum: f64 = result.eternal_rule.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_emergent_eternal_rules() {
        let result = EternalGovernanceResult {
            eternal_rule: vec![0.3, 0.4, 0.3],
            rule_convergence: 0.0,
            proposals_processed: 10,
            proposals_approved: 5,
            total_governance_weight: 1.0,
            active_governors: 3,
            coherence_maintained: true,
            epochs: 5,
            rule_trajectory: vec![],
        };
        let rules = emergent_eternal_rules(&result);
        assert_eq!(rules, vec![0.3, 0.4, 0.3]);
    }

    // ─── Result Display Tests ─────────────────────────────────────────────

    #[test]
    fn test_eternal_governance_result_summary() {
        let result = EternalGovernanceResult {
            eternal_rule: vec![0.3, 0.4, 0.3],
            rule_convergence: -0.01,
            proposals_processed: 10,
            proposals_approved: 5,
            total_governance_weight: 1.0,
            active_governors: 3,
            coherence_maintained: true,
            epochs: 5,
            rule_trajectory: vec![],
        };
        let summary = result.summary();
        assert!(summary.contains("rule_dim=3"));
        assert!(summary.contains("governors=3"));
    }

    #[test]
    fn test_eternal_governance_result_display() {
        let result = EternalGovernanceResult {
            eternal_rule: vec![0.5, 0.5],
            rule_convergence: 0.0,
            proposals_processed: 0,
            proposals_approved: 0,
            total_governance_weight: 0.0,
            active_governors: 0,
            coherence_maintained: true,
            epochs: 0,
            rule_trajectory: vec![],
        };
        let display = format!("{}", result);
        assert!(!display.is_empty());
    }

    // ─── LCG Random Tests ─────────────────────────────────────────────────

    #[test]
    fn test_lcg_next_deterministic() {
        let mut s1: u64 = 42;
        let mut s2: u64 = 42;
        assert_eq!(lcg_next(&mut s1), lcg_next(&mut s2));
    }

    #[test]
    fn test_random_uniform_range() {
        let mut s: u64 = 42;
        for _ in 0..100 {
            let v = random_uniform(&mut s);
            assert!(v >= 0.0 && v <= 1.0);
        }
    }
}
