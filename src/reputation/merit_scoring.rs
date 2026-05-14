//! Merit Scoring — Meritocratic reputation scoring based on technical contributions.
//!
//! Computes reputation scores from:
//! - Code contributions (commits, PRs, reviews)
//! - Compute credits (work performed)
//! - Governance participation
//! - Community impact
//!
//! Zero financial logic: scores represent technical reputation only.

use std::collections::{HashMap, VecDeque};

// ─── Errors ───

#[derive(Debug, Clone)]
pub enum MeritError {
    NodeNotFound(String),
    InvalidWeight(String),
    ScoreOverflow,
}

impl std::fmt::Display for MeritError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            Self::InvalidWeight(msg) => write!(f, "Invalid weight: {}", msg),
            Self::ScoreOverflow => write!(f, "Score overflow detected"),
        }
    }
}

impl std::error::Error for MeritError {}

// ─── Config ───

#[derive(Debug, Clone)]
pub struct MeritConfig {
    pub contribution_weight: f64,
    pub compute_weight: f64,
    pub governance_weight: f64,
    pub review_weight: f64,
    pub decay_rate: f64,
    pub max_score: f64,
    pub min_score: f64,
}

impl Default for MeritConfig {
    fn default() -> Self {
        Self {
            contribution_weight: 0.35,
            compute_weight: 0.30,
            governance_weight: 0.20,
            review_weight: 0.15,
            decay_rate: 0.001,
            max_score: 1000.0,
            min_score: 0.0,
        }
    }
}

// ─── Contribution Record ───

#[derive(Debug, Clone)]
pub enum ContributionKind {
    Code,
    Documentation,
    Review,
    ComputeWork,
    GovernanceVote,
    BugReport,
}

impl std::fmt::Display for ContributionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Code => write!(f, "code"),
            Self::Documentation => write!(f, "documentation"),
            Self::Review => write!(f, "review"),
            Self::ComputeWork => write!(f, "compute_work"),
            Self::GovernanceVote => write!(f, "governance_vote"),
            Self::BugReport => write!(f, "bug_report"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContributionRecord {
    pub node_id: String,
    pub kind: ContributionKind,
    pub value: f64,
    pub timestamp_ms: u64,
}

// ─── Merit Profile ───

#[derive(Debug, Clone)]
pub struct MeritProfile {
    pub node_id: String,
    pub total_score: f64,
    pub contribution_score: f64,
    pub compute_score: f64,
    pub governance_score: f64,
    pub review_score: f64,
    pub total_contributions: u64,
    pub last_update_ms: u64,
}

impl MeritProfile {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            total_score: 0.0,
            contribution_score: 0.0,
            compute_score: 0.0,
            governance_score: 0.0,
            review_score: 0.0,
            total_contributions: 0,
            last_update_ms: 0,
        }
    }

    pub fn rank_percentile(&self, all_scores: &[f64]) -> f64 {
        if all_scores.is_empty() {
            return 0.0;
        }
        let below = all_scores.iter().filter(|&&s| s < self.total_score).count();
        below as f64 / all_scores.len() as f64
    }
}

// ─── Stats ───

#[derive(Debug, Clone)]
pub struct MeritStats {
    pub total_nodes: u64,
    pub total_contributions: u64,
    pub avg_score: f64,
    pub max_score_observed: f64,
    pub last_decay_ms: u64,
}

impl Default for MeritStats {
    fn default() -> Self {
        Self {
            total_nodes: 0,
            total_contributions: 0,
            avg_score: 0.0,
            max_score_observed: 0.0,
            last_decay_ms: 0,
        }
    }
}

// ─── Scorer ───

pub struct MeritScorer {
    config: MeritConfig,
    profiles: HashMap<String, MeritProfile>,
    history: VecDeque<ContributionRecord>,
    stats: MeritStats,
}

impl MeritScorer {
    pub fn new(config: MeritConfig) -> Self {
        Self {
            config,
            profiles: HashMap::new(),
            history: VecDeque::new(),
            stats: MeritStats::default(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(MeritConfig::default())
    }

    pub fn record_contribution(
        &mut self,
        node_id: String,
        kind: ContributionKind,
        value: f64,
    ) -> Result<(), MeritError> {
        let now = current_timestamp_ms();
        let record = ContributionRecord {
            node_id: node_id.clone(),
            kind: kind.clone(),
            value,
            timestamp_ms: now,
        };

        // Ensure profile exists
        if !self.profiles.contains_key(&node_id) {
            self.profiles.insert(node_id.clone(), MeritProfile::new(node_id.clone()));
            self.stats.total_nodes += 1;
        }

        // Apply weighted score
        let weighted = match &kind {
            ContributionKind::Code | ContributionKind::Documentation => {
                value * self.config.contribution_weight
            }
            ContributionKind::ComputeWork => {
                value * self.config.compute_weight
            }
            ContributionKind::GovernanceVote => {
                value * self.config.governance_weight
            }
            ContributionKind::Review => {
                value * self.config.review_weight
            }
            ContributionKind::BugReport => {
                value * self.config.contribution_weight * 0.5
            }
        };

        let profile = self.profiles.get_mut(&node_id).unwrap();
        profile.total_contributions += 1;
        profile.last_update_ms = now;

        match &kind {
            ContributionKind::Code | ContributionKind::Documentation | ContributionKind::BugReport => {
                profile.contribution_score += weighted;
            }
            ContributionKind::ComputeWork => {
                profile.compute_score += weighted;
            }
            ContributionKind::GovernanceVote => {
                profile.governance_score += weighted;
            }
            ContributionKind::Review => {
                profile.review_score += weighted;
            }
        }

        profile.total_score += weighted;
        profile.total_score = profile.total_score.min(self.config.max_score);
        let final_score = profile.total_score;

        self.stats.total_contributions += 1;
        self.update_avg_score();
        if final_score > self.stats.max_score_observed {
            self.stats.max_score_observed = final_score;
        }

        self.history.push_back(record);
        if self.history.len() > 10_000 {
            self.history.pop_front();
        }

        Ok(())
    }

    pub fn apply_decay(&mut self) {
        let now = current_timestamp_ms();
        for profile in self.profiles.values_mut() {
            let elapsed = now.saturating_sub(profile.last_update_ms);
            let decay = 1.0 - (self.config.decay_rate * elapsed as f64 / 1000.0);
            let decay = decay.clamp(0.0, 1.0);

            profile.contribution_score *= decay;
            profile.compute_score *= decay;
            profile.governance_score *= decay;
            profile.review_score *= decay;
            profile.total_score =
                (profile.contribution_score + profile.compute_score
                    + profile.governance_score
                    + profile.review_score)
                    .min(self.config.max_score)
                    .max(self.config.min_score);
        }
        self.stats.last_decay_ms = now;
        self.update_avg_score();
    }

    pub fn get_profile(&self, node_id: &str) -> Option<&MeritProfile> {
        self.profiles.get(node_id)
    }

    pub fn get_ranking(&self) -> Vec<&MeritProfile> {
        let mut profiles: Vec<_> = self.profiles.values().collect();
        profiles.sort_by(|a, b| b.total_score.partial_cmp(&a.total_score).unwrap_or(std::cmp::Ordering::Equal));
        profiles
    }

    pub fn get_percentile(&self, node_id: &str) -> Option<f64> {
        let profile = self.profiles.get(node_id)?;
        let scores: Vec<f64> = self.profiles.values().map(|p| p.total_score).collect();
        Some(profile.rank_percentile(&scores))
    }

    pub fn get_stats(&self) -> &MeritStats {
        &self.stats
    }

    pub fn get_config(&self) -> &MeritConfig {
        &self.config
    }

    fn update_avg_score(&mut self) {
        if self.profiles.is_empty() {
            self.stats.avg_score = 0.0;
            return;
        }
        let sum: f64 = self.profiles.values().map(|p| p.total_score).sum();
        self.stats.avg_score = sum / self.profiles.len() as f64;
    }
}

impl Default for MeritScorer {
    fn default() -> Self {
        Self::with_defaults()
    }
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scorer_creation() {
        let s = MeritScorer::with_defaults();
        assert_eq!(s.get_stats().total_nodes, 0);
    }

    #[test]
    fn test_record_contribution() {
        let mut s = MeritScorer::with_defaults();
        s.record_contribution("n1".to_string(), ContributionKind::Code, 10.0)
            .unwrap();
        let profile = s.get_profile("n1").unwrap();
        assert!((profile.total_score - 10.0 * 0.35).abs() < 0.01);
    }

    #[test]
    fn test_multiple_contributions() {
        let mut s = MeritScorer::with_defaults();
        s.record_contribution("n1".to_string(), ContributionKind::Code, 10.0)
            .unwrap();
        s.record_contribution("n1".to_string(), ContributionKind::ComputeWork, 20.0)
            .unwrap();
        let profile = s.get_profile("n1").unwrap();
        assert_eq!(profile.total_contributions, 2);
    }

    #[test]
    fn test_score_clamping() {
        let mut s = MeritScorer::with_defaults();
        s.record_contribution("n1".to_string(), ContributionKind::Code, 5000.0)
            .unwrap();
        let profile = s.get_profile("n1").unwrap();
        assert!(profile.total_score <= 1000.0);
    }

    #[test]
    fn test_ranking() {
        let mut s = MeritScorer::with_defaults();
        s.record_contribution("n1".to_string(), ContributionKind::Code, 10.0)
            .unwrap();
        s.record_contribution("n2".to_string(), ContributionKind::Code, 20.0)
            .unwrap();
        let ranking = s.get_ranking();
        assert_eq!(ranking[0].node_id, "n2");
    }

    #[test]
    fn test_percentile() {
        let mut s = MeritScorer::with_defaults();
        s.record_contribution("n1".to_string(), ContributionKind::Code, 10.0)
            .unwrap();
        s.record_contribution("n2".to_string(), ContributionKind::Code, 20.0)
            .unwrap();
        s.record_contribution("n3".to_string(), ContributionKind::Code, 30.0)
            .unwrap();
        let p = s.get_percentile("n2").unwrap();
        assert!(p > 0.0 && p <= 1.0);
    }

    #[test]
    fn test_decay() {
        let mut s = MeritScorer::with_defaults();
        s.record_contribution("n1".to_string(), ContributionKind::Code, 10.0)
            .unwrap();
        let before = s.get_profile("n1").unwrap().total_score;
        s.apply_decay();
        let after = s.get_profile("n1").unwrap().total_score;
        assert!(after <= before);
    }

    #[test]
    fn test_stats_tracking() {
        let mut s = MeritScorer::with_defaults();
        s.record_contribution("n1".to_string(), ContributionKind::Code, 10.0)
            .unwrap();
        assert_eq!(s.get_stats().total_nodes, 1);
        assert_eq!(s.get_stats().total_contributions, 1);
    }

    #[test]
    fn test_contribution_kind_display() {
        let k = ContributionKind::Code;
        assert_eq!(k.to_string(), "code");
    }

    #[test]
    fn test_error_display() {
        let e = MeritError::NodeNotFound("x".to_string());
        assert!(!e.to_string().is_empty());
    }
}
