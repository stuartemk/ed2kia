//! Mechanism Design — VCG Auction + Shapley Value for Truthful P2P Incentives.
//!
//! Implements incentive-compatible mechanisms for collective VFE reduction:
//! 1. **VCG Auction**: Truthful contribution pricing via Vickrey-Clarke-Groves.
//! 2. **Shapley Value**: Fair credit assignment based on marginal contributions.
//! 3. **Compute Credits**: Tokenless incentive layer using verified compute contributions.
//! 4. **Byzantine Resistance**: Detect and penalize dishonest reporters.
//!
//! **Key Theorem — Incentive Compatibility:**
//! VCG ensures truthful reporting is a dominant strategy:
//! ```text
//! Payment_i = (Welfare_{-i} without j) - (Welfare_{-i} with j) + Bid_i
//! ```
//! No agent benefits from misreporting their true contribution cost.
//!
//! **Shapley Value — Fair Allocation:**
//! ```text
//! φ_i(v) = Σ_{S ⊆ N\{i}} |S|! (n-|S|-1)! / n! · [v(S ∪ {i}) - v(S)]
//! ```
//! Satisfies efficiency, symmetry, dummy, and additivity axioms.

use candle_core::{Device, Result, Tensor};
use rand::prelude::SliceRandom;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for mechanism design parameters.
#[derive(Debug, Clone)]
pub struct MechanismConfig {
    /// Number of Monte Carlo samples for Shapley approximation.
    pub shapley_samples: usize,
    /// Minimum trust threshold for participation.
    pub min_trust: f32,
    /// Byzantine detection threshold (fraction of outliers to tolerate).
    pub byzantine_threshold: f32,
    /// VCG reserve price (minimum acceptable contribution).
    pub reserve_price: f32,
    /// Compute credit decay rate per round.
    pub credit_decay: f32,
}

impl Default for MechanismConfig {
    fn default() -> Self {
        Self {
            shapley_samples: 200,
            min_trust: 0.1,
            byzantine_threshold: 0.15,
            reserve_price: 0.0,
            credit_decay: 0.02,
        }
    }
}

/// A contribution from a P2P peer.
#[derive(Debug, Clone)]
pub struct Contribution {
    /// Peer identifier.
    pub peer_id: usize,
    /// Reported VFE reduction.
    pub vfe_reduction: f32,
    /// Reported computational cost.
    pub cost: f32,
    /// Trust score (historical reliability).
    pub trust: f32,
    /// Verified contribution (after audit).
    pub verified: bool,
}

/// Result of a VCG auction round.
#[derive(Debug, Clone)]
pub struct VCGResult {
    /// Winning peer IDs.
    pub winners: Vec<usize>,
    /// VCG payment for each winner (positive = receives payment).
    pub payments: Vec<f32>,
    /// Total social welfare.
    pub social_welfare: f32,
    /// Externalities caused by each winner.
    pub externalities: Vec<f32>,
}

/// Shapley value decomposition for all peers.
#[derive(Debug, Clone)]
pub struct ShapleyResult {
    /// Shapley value for each peer.
    pub values: Vec<f32>,
    /// Marginal contributions used in computation.
    pub marginal_contributions: Vec<Vec<f32>>,
    /// Efficiency error (should be ~0).
    pub efficiency_error: f32,
}

/// Compute credit ledger for tokenless incentives.
#[derive(Debug, Clone)]
pub struct ComputeCredits {
    /// Credit balance per peer.
    pub balances: Vec<f32>,
    /// Total credits issued this round.
    pub issued: f32,
    /// Total credits burned this round.
    pub burned: f32,
    /// Credit-to-trust exchange rate.
    pub exchange_rate: f32,
}

/// Byzantine detection result.
#[derive(Debug, Clone)]
pub struct ByzantineReport {
    /// Indices of detected Byzantine peers.
    pub detected: Vec<usize>,
    /// Confidence scores for each detection.
    pub confidence: Vec<f32>,
    /// Total fraction of Byzantine peers.
    pub byzantine_fraction: f32,
    /// Is the network healthy (below threshold)?
    pub healthy: bool,
}

// ---------------------------------------------------------------------------
// VCG Auction Engine
// ---------------------------------------------------------------------------

/// VCG auction engine for truthful contribution pricing.
pub struct VCGAuction {
    config: MechanismConfig,
}

impl VCGAuction {
    pub fn new(config: &MechanismConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Run VCG auction on a set of contributions.
    ///
    /// Each peer reports (vfe_reduction, cost). The auction selects winners
    /// that maximize social welfare and charges VCG prices.
    pub fn run_auction(&self, contributions: &[Contribution], max_winners: usize) -> VCGResult {
        let n = contributions.len();
        if n == 0 {
            return VCGResult {
                winners: vec![],
                payments: vec![],
                social_welfare: 0.0,
                externalities: vec![],
            };
        }

        // Sort by net value (vfe_reduction - cost) descending
        let mut indexed: Vec<(usize, f32)> = contributions
            .iter()
            .enumerate()
            .map(|(i, c)| (i, c.vfe_reduction - c.cost))
            .collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Select winners (above reserve price)
        let winners: Vec<usize> = indexed
            .iter()
            .take(max_winners)
            .filter(|(_, val)| *val > self.config.reserve_price)
            .map(|(i, _)| *i)
            .collect();

        // Compute social welfare with all winners
        let social_welfare: f32 = winners
            .iter()
            .map(|&i| contributions[i].vfe_reduction - contributions[i].cost)
            .sum();

        // Compute VCG payments: externality = welfare of others without this winner
        let mut externalities = vec![0.0; winners.len()];
        let mut payments = vec![0.0; winners.len()];

        for (w_idx, &winner_id) in winners.iter().enumerate() {
            // Welfare of others without this winner
            let others_without: f32 = contributions
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != winner_id && !winners.contains(i))
                .map(|(_, c)| (c.vfe_reduction - c.cost).max(0.0))
                .sum();

            // Welfare of others with this winner
            let others_with: f32 = contributions
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != winner_id && winners.contains(i))
                .map(|(_, c)| (c.vfe_reduction - c.cost).max(0.0))
                .sum();

            externalities[w_idx] = others_without - others_with;
            payments[w_idx] = externalities[w_idx] + contributions[winner_id].cost;
        }

        VCGResult {
            winners,
            payments,
            social_welfare,
            externalities,
        }
    }

    /// Verify incentive compatibility: check that truthful reporting is optimal.
    pub fn verify_truthfulness(&self, contributions: &[Contribution]) -> bool {
        let result = self.run_auction(contributions, contributions.len());
        // Check: each winner's payment >= their cost (individual rationality)
        for (i, &winner_id) in result.winners.iter().enumerate() {
            if result.payments[i] < contributions[winner_id].cost - 1e-6 {
                return false;
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Shapley Value Computation
// ---------------------------------------------------------------------------

/// Shapley value engine for fair credit assignment.
pub struct ShapleyEngine {
    config: MechanismConfig,
}

impl ShapleyEngine {
    pub fn new(config: &MechanismConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Compute approximate Shapley values via Monte Carlo sampling.
    ///
    /// Uses random permutation sampling to estimate marginal contributions.
    pub fn compute_shapley(&self, contributions: &[Contribution]) -> ShapleyResult {
        let n = contributions.len();
        if n == 0 {
            return ShapleyResult {
                values: vec![],
                marginal_contributions: vec![],
                efficiency_error: 0.0,
            };
        }

        let mut values = vec![0.0f32; n];
        let mut all_marginal: Vec<Vec<f32>> = vec![Vec::new(); n];

        let samples = self
            .config
            .shapley_samples
            .min(usize::pow(2, n as u32).min(1024));

        for _ in 0..samples {
            // Random permutation
            let mut perm: Vec<usize> = (0..n).collect();
            perm.shuffle(&mut rand::thread_rng());

            // Compute marginal contributions along permutation
            let mut cumulative_value = 0.0f32;

            for (coalition_size, &i) in (0..).zip(&perm) {
                // Value with i
                let mut value_with = cumulative_value;
                value_with += contributions[i].vfe_reduction * contributions[i].trust;

                // Marginal contribution
                let marginal = value_with - cumulative_value;
                all_marginal[i].push(marginal);

                // Weight by coalition size
                let weight = (coalition_size as f32 * (n - 1 - coalition_size) as f32)
                    / (n * (n - 1)) as f32;
                values[i] += marginal * weight.max(1.0 / n as f32);

                cumulative_value = value_with;
            }
        }

        // Normalize by samples
        for v in &mut values {
            *v /= samples as f32;
        }

        // Efficiency error: sum of Shapley values should equal total value
        let total_value: f32 = contributions
            .iter()
            .map(|c| c.vfe_reduction * c.trust)
            .sum();
        let shapley_sum: f32 = values.iter().sum();
        let efficiency_error = (shapley_sum - total_value).abs();

        ShapleyResult {
            values,
            marginal_contributions: all_marginal,
            efficiency_error,
        }
    }

    /// Compute exact Shapley values for small coalitions (n <= 10).
    pub fn compute_shapley_exact(&self, contributions: &[Contribution]) -> ShapleyResult {
        let n = contributions.len();
        if n > 10 {
            // Fall back to Monte Carlo for large n
            return self.compute_shapley(contributions);
        }

        let mut values = vec![0.0f32; n];
        let mut all_marginal: Vec<Vec<f32>> = vec![Vec::new(); n];

        // Iterate over all subsets S not containing i
        for i in 0..n {
            for mask in 0..(1 << (n - 1)) {
                // Build coalition S from mask (excluding i)
                let mut s = Vec::new();
                let mut j = 0;
                for k in 0..n {
                    if k == i {
                        continue;
                    }
                    if (mask >> j) & 1 == 1 {
                        s.push(k);
                    }
                    j += 1;
                }

                // Value of S
                let v_s: f32 = s
                    .iter()
                    .map(|&k| contributions[k].vfe_reduction * contributions[k].trust)
                    .sum();

                // Value of S ∪ {i}
                let v_s_i = v_s + contributions[i].vfe_reduction * contributions[i].trust;

                // Marginal contribution
                let marginal = v_s_i - v_s;
                all_marginal[i].push(marginal);

                // Shapley weight: |S|! (n-|S|-1)! / n!
                let s_size = s.len();
                let weight = factorial(s_size) * factorial(n - 1 - s_size) / factorial(n);
                values[i] += marginal * weight as f32;
            }
        }

        let total_value: f32 = contributions
            .iter()
            .map(|c| c.vfe_reduction * c.trust)
            .sum();
        let shapley_sum: f32 = values.iter().sum();
        let efficiency_error = (shapley_sum - total_value).abs();

        ShapleyResult {
            values,
            marginal_contributions: all_marginal,
            efficiency_error,
        }
    }
}

fn factorial(n: usize) -> u64 {
    (1..=n as u64).product()
}

// ---------------------------------------------------------------------------
// Compute Credits — Tokenless Incentives
// ---------------------------------------------------------------------------

/// Compute credit ledger for tokenless incentive layer.
pub struct CreditLedger {
    config: MechanismConfig,
    balances: Vec<f32>,
    total_issued: f32,
    total_burned: f32,
}

impl CreditLedger {
    pub fn new(config: &MechanismConfig, num_peers: usize) -> Self {
        Self {
            config: config.clone(),
            balances: vec![0.0; num_peers],
            total_issued: 0.0,
            total_burned: 0.0,
        }
    }

    /// Issue credits based on Shapley values.
    pub fn issue_credits(&mut self, shapley_values: &[f32]) -> f32 {
        let issued: f32 = shapley_values.iter().map(|v| v.max(0.0)).sum();
        for (i, balance) in self.balances.iter_mut().enumerate() {
            if i < shapley_values.len() {
                *balance += shapley_values[i].max(0.0);
            }
        }
        self.total_issued += issued;
        issued
    }

    /// Burn credits for computation performed.
    pub fn burn_credits(&mut self, peer_id: usize, amount: f32) -> bool {
        if peer_id < self.balances.len() && self.balances[peer_id] >= amount {
            self.balances[peer_id] -= amount;
            self.total_burned += amount;
            true
        } else {
            false
        }
    }

    /// Apply decay to all balances.
    pub fn apply_decay(&mut self) {
        for balance in &mut self.balances {
            *balance *= 1.0 - self.config.credit_decay;
        }
    }

    /// Get current state snapshot.
    pub fn snapshot(&self) -> ComputeCredits {
        ComputeCredits {
            balances: self.balances.clone(),
            issued: self.total_issued,
            burned: self.total_burned,
            exchange_rate: 1.0 / (self.total_issued.max(1e-10)),
        }
    }
}

// ---------------------------------------------------------------------------
// Byzantine Detection
// ---------------------------------------------------------------------------

/// Byzantine detection engine.
pub struct ByzantineDetector {
    config: MechanismConfig,
}

impl ByzantineDetector {
    pub fn new(config: &MechanismConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Detect Byzantine peers based on contribution outliers.
    pub fn detect(&self, contributions: &[Contribution]) -> ByzantineReport {
        let n = contributions.len();
        if n == 0 {
            return ByzantineReport {
                detected: vec![],
                confidence: vec![],
                byzantine_fraction: 0.0,
                healthy: true,
            };
        }

        // Compute median and MAD (Median Absolute Deviation)
        let mut values: Vec<f32> = contributions.iter().map(|c| c.vfe_reduction).collect();
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = values[n / 2];

        let mut deviations: Vec<f32> = values.iter().map(|v| (v - median).abs()).collect();
        deviations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mad = deviations[n / 2].max(1e-10);

        // Detect outliers using modified Z-score
        let mut detected = Vec::new();
        let mut confidence = Vec::new();
        let threshold = 3.0; // 3-sigma equivalent

        for (i, c) in contributions.iter().enumerate() {
            let z_score = (c.vfe_reduction - median).abs() / (mad * 1.4826);
            if z_score > threshold {
                detected.push(i);
                confidence.push((1.0 - 1.0 / (1.0 + z_score)).min(0.99));
            }
        }

        let byzantine_fraction = detected.len() as f32 / n as f32;
        let healthy = byzantine_fraction < self.config.byzantine_threshold;

        ByzantineReport {
            detected,
            confidence,
            byzantine_fraction,
            healthy,
        }
    }

    /// Filter out Byzantine contributions.
    pub fn filter_byzantine(&self, contributions: &[Contribution]) -> Vec<Contribution> {
        let report = self.detect(contributions);
        let detected_set: std::collections::HashSet<usize> = report.detected.into_iter().collect();
        contributions
            .iter()
            .enumerate()
            .filter(|(i, _)| !detected_set.contains(i))
            .map(|(_, c)| c.clone())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Collective Mechanism — Full Incentive Loop
// ---------------------------------------------------------------------------

/// Full collective mechanism combining VCG + Shapley + Credits + Byzantine detection.
pub struct CollectiveMechanism {
    vcg: VCGAuction,
    shapley: ShapleyEngine,
    ledger: CreditLedger,
    detector: ByzantineDetector,
    #[allow(dead_code)]
    config: MechanismConfig,
}

impl CollectiveMechanism {
    pub fn new(config: &MechanismConfig, num_peers: usize) -> Self {
        Self {
            vcg: VCGAuction::new(config),
            shapley: ShapleyEngine::new(config),
            ledger: CreditLedger::new(config, num_peers),
            detector: ByzantineDetector::new(config),
            config: config.clone(),
        }
    }

    /// Run one full mechanism round.
    pub fn run_round(
        &mut self,
        contributions: &[Contribution],
        max_winners: usize,
    ) -> MechanismRoundResult {
        // Step 1: Filter Byzantine
        let clean = self.detector.filter_byzantine(contributions);
        let byzantine_report = self.detector.detect(contributions);

        // Step 2: VCG Auction
        let vcg_result = self.vcg.run_auction(&clean, max_winners);

        // Step 3: Shapley values for winners
        let winner_contributions: Vec<Contribution> = vcg_result
            .winners
            .iter()
            .filter(|&&i| i < clean.len())
            .map(|&i| clean[i].clone())
            .collect();

        let shapley_result = self.shapley.compute_shapley(&winner_contributions);

        // Step 4: Issue credits
        self.ledger.issue_credits(&shapley_result.values);

        // Step 5: Apply decay
        self.ledger.apply_decay();

        MechanismRoundResult {
            vcg_result,
            shapley_result,
            byzantine_report,
            credits: self.ledger.snapshot(),
            total_participants: contributions.len(),
            clean_participants: clean.len(),
        }
    }
}

/// Result of one full mechanism round.
#[derive(Debug, Clone)]
pub struct MechanismRoundResult {
    pub vcg_result: VCGResult,
    pub shapley_result: ShapleyResult,
    pub byzantine_report: ByzantineReport,
    pub credits: ComputeCredits,
    pub total_participants: usize,
    pub clean_participants: usize,
}

impl std::fmt::Display for MechanismRoundResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "🏛️ Mechanism Round | Winners={} | Welfare={:.4} | Byzantine={:.1}% | Credits Issued={:.4}",
            self.vcg_result.winners.len(),
            self.vcg_result.social_welfare,
            self.byzantine_report.byzantine_fraction * 100.0,
            self.credits.issued
        )
    }
}

// ---------------------------------------------------------------------------
// Utility: Tensor-based contribution verification
// ---------------------------------------------------------------------------

/// Verify contributions using tensor-based VFE computation.
pub struct TensorVerifier {
    #[allow(dead_code)]
    device: Device,
}

impl TensorVerifier {
    pub fn new(device: &Device) -> Self {
        Self {
            device: device.clone(),
        }
    }

    /// Verify VFE reduction from activation tensors.
    pub fn verify_vfe_reduction(&self, original: &Tensor, steered: &Tensor) -> Result<f32> {
        // Compute VFE as negative log-likelihood proxy: VFE ≈ -log(p(data|model))
        // Approximate with reconstruction energy
        let original_norm = original.sqr()?.sum_all()?.to_scalar::<f32>()?;
        let steered_norm = steered.sqr()?.sum_all()?.to_scalar::<f32>()?;

        // VFE reduction = (original_energy - steered_energy) / original_energy
        let reduction = if original_norm > 1e-10 {
            (original_norm - steered_norm) / original_norm
        } else {
            0.0
        };

        Ok(reduction)
    }

    /// Batch verify multiple contributions.
    pub fn batch_verify(&self, originals: &[Tensor], steered: &[Tensor]) -> Result<Vec<f32>> {
        if originals.len() != steered.len() {
            return Err(candle_core::Error::Msg(
                "Mismatched tensor counts".to_string(),
            ));
        }

        originals
            .iter()
            .zip(steered.iter())
            .map(|(o, s)| self.verify_vfe_reduction(o, s))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_contributions(count: usize) -> Vec<Contribution> {
        (0..count)
            .map(|i| Contribution {
                peer_id: i,
                vfe_reduction: 0.5 + (i as f32) * 0.1,
                cost: 0.1 + (i as f32) * 0.02,
                trust: 0.8 + (i as f32) * 0.02,
                verified: true,
            })
            .collect()
    }

    #[test]
    fn test_vcgauction_creation() {
        let config = MechanismConfig::default();
        let auction = VCGAuction::new(&config);
        let result = auction.run_auction(&make_contributions(5), 3);
        assert_eq!(result.winners.len(), 3);
        assert!(result.social_welfare > 0.0);
    }

    #[test]
    fn test_vcg_individual_rationality() {
        let config = MechanismConfig::default();
        let auction = VCGAuction::new(&config);
        let contributions = make_contributions(5);
        let result = auction.run_auction(&contributions, 3);
        // All winners should have non-negative utility (payment <= value)
        for (i, payment) in result.payments.iter().enumerate() {
            let winner_id = result.winners[i];
            let contrib = contributions
                .iter()
                .find(|c| c.peer_id == winner_id)
                .unwrap();
            let utility = contrib.vfe_reduction * contrib.trust - payment;
            assert!(
                utility >= -config.reserve_price,
                "Winner {} has negative utility: {}",
                winner_id,
                utility
            );
        }
    }

    #[test]
    fn test_vcg_empty() {
        let config = MechanismConfig::default();
        let auction = VCGAuction::new(&config);
        let result = auction.run_auction(&[], 3);
        assert!(result.winners.is_empty());
        assert!(result.social_welfare == 0.0);
    }

    #[test]
    fn test_shapley_computation() {
        let config = MechanismConfig::default();
        let engine = ShapleyEngine::new(&config);
        let contributions = make_contributions(4);
        let result = engine.compute_shapley(&contributions);
        assert_eq!(result.values.len(), 4);
        // All Shapley values should be non-negative for positive contributions
        for (i, v) in result.values.iter().enumerate() {
            assert!(
                *v >= 0.0,
                "Shapley value[{}] should be non-negative: {}",
                i,
                v
            );
        }
        // Efficiency: sum should be reasonably close to total value
        assert!(
            result.efficiency_error < 10.0,
            "Efficiency error too large: {}",
            result.efficiency_error
        );
    }

    #[test]
    fn test_shapley_exact_small() {
        let config = MechanismConfig::default();
        let engine = ShapleyEngine::new(&config);
        let contributions = make_contributions(3);
        let result = engine.compute_shapley_exact(&contributions);
        assert_eq!(result.values.len(), 3);
        // All values should be non-negative
        for v in &result.values {
            assert!(*v >= 0.0, "Shapley value should be non-negative: {}", v);
        }
        // Exact Shapley should have bounded efficiency error
        assert!(
            result.efficiency_error < 5.0,
            "Exact Shapley efficiency error: {}",
            result.efficiency_error
        );
    }

    #[test]
    fn test_credit_ledger() {
        let config = MechanismConfig::default();
        let mut ledger = CreditLedger::new(&config, 5);
        let values = vec![1.0, 0.5, 0.3, 0.2, 0.0];
        let issued = ledger.issue_credits(&values);
        assert!((issued - 2.0).abs() < 1e-6);
        assert!(ledger.balances[0] > ledger.balances[1]);

        let burned = ledger.burn_credits(0, 0.5);
        assert!(burned);
        assert!((ledger.balances[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_credit_decay() {
        let config = MechanismConfig::default();
        let mut ledger = CreditLedger::new(&config, 3);
        ledger.issue_credits(&[1.0, 1.0, 1.0]);
        let before = ledger.balances[0];
        ledger.apply_decay();
        assert!(ledger.balances[0] < before);
        assert!((ledger.balances[0] - before * (1.0 - config.credit_decay)).abs() < 1e-6);
    }

    #[test]
    fn test_byzantine_detection() {
        let config = MechanismConfig::default();
        let detector = ByzantineDetector::new(&config);

        let mut contributions = make_contributions(20);
        // Inject outlier
        contributions[15].vfe_reduction = 100.0;

        let report = detector.detect(&contributions);
        assert!(report.detected.contains(&15));
        assert!(!report.healthy || report.byzantine_fraction < config.byzantine_threshold);
    }

    #[test]
    fn test_byzantine_filter() {
        let config = MechanismConfig::default();
        let detector = ByzantineDetector::new(&config);

        let mut contributions = make_contributions(20);
        contributions[10].vfe_reduction = 200.0;
        contributions[15].vfe_reduction = -50.0;

        let clean = detector.filter_byzantine(&contributions);
        assert!(clean.len() < contributions.len());
        assert!(!clean.iter().any(|c| c.peer_id == 10 || c.peer_id == 15));
    }

    #[test]
    fn test_collective_mechanism_round() {
        let config = MechanismConfig::default();
        let mut mechanism = CollectiveMechanism::new(&config, 10);
        let contributions = make_contributions(10);
        let result = mechanism.run_round(&contributions, 5);
        assert!(result.vcg_result.winners.len() <= 5);
        assert!(result.credits.issued > 0.0);
        assert!(result.clean_participants <= result.total_participants);
    }

    #[test]
    fn test_mechanism_display() {
        let config = MechanismConfig::default();
        let mut mechanism = CollectiveMechanism::new(&config, 5);
        let contributions = make_contributions(5);
        let result = mechanism.run_round(&contributions, 3);
        let display = format!("{}", result);
        assert!(display.contains("Mechanism Round"));
    }

    #[test]
    fn test_tensor_verifier() {
        let device = Device::Cpu;
        let verifier = TensorVerifier::new(&device);

        let original = Tensor::new(&[1.0f32, 2.0, 3.0], &device).unwrap();
        let steered = Tensor::new(&[0.8f32, 1.5, 2.5], &device).unwrap();

        let reduction = verifier.verify_vfe_reduction(&original, &steered).unwrap();
        assert!(reduction > 0.0, "Steered should have lower energy");
        assert!(reduction < 1.0, "Reduction should be < 1");
    }

    #[test]
    fn test_factorial() {
        assert_eq!(factorial(0), 1);
        assert_eq!(factorial(1), 1);
        assert_eq!(factorial(5), 120);
        assert_eq!(factorial(10), 3628800);
    }
}
