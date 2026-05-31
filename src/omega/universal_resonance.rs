//! Universal Resonance and Personal Echo — Sprint 62
//!
//! Implements **R_universal(t)** — the Universal Resonance Field that aggregates
//! the weighted Resonance Field by human participation. Also defines the
//! **Personal Echo** structure: a lightweight representation of each user's
//! cognitive-ethical fingerprint on the Moral Manifold.
//!
//! # Universal Resonance Formula
//!
//! ```text
//! R_universal(t) = Σ [ p_i * echo_i.coherence * echo_i.ethical_alignment ] / Σ p_i
//! ```
//!
//! Where:
//! - `p_i` — Participation weight of user i
//! - `echo_i` — Personal Echo of user i
//!
//! # Personal Echo
//!
//! Each echo captures the user's contribution to the collective meta-ethical
//! hypothesis space, enabling the network to generate higher-order coordination
//! insights from aggregated collective intuition.

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors in resonance computation or echo management.
#[derive(Debug, Clone, PartialEq)]
pub enum ResonanceError {
    /// Participation weight must be non-negative.
    NegativeParticipation(f64),
    /// No echoes available for resonance computation.
    NoEchoesAvailable,
    /// User ID not found in echo registry.
    UserIdNotFound(u64),
    /// Coherence value outside valid range [0.0, 1.0].
    CoherenceOutOfRange(f64),
    /// Ethical alignment outside valid range [-1.0, 1.0].
    AlignmentOutOfRange(f64),
}

impl std::fmt::Display for ResonanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResonanceError::NegativeParticipation(val) => {
                write!(f, "Participation weight {} must be non-negative", val)
            }
            ResonanceError::NoEchoesAvailable => {
                write!(f, "No echoes available for resonance computation")
            }
            ResonanceError::UserIdNotFound(id) => {
                write!(f, "User ID {} not found in echo registry", id)
            }
            ResonanceError::CoherenceOutOfRange(val) => {
                write!(f, "Coherence {} outside valid range [0.0, 1.0]", val)
            }
            ResonanceError::AlignmentOutOfRange(val) => {
                write!(
                    f,
                    "Ethical alignment {} outside valid range [-1.0, 1.0]",
                    val
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Personal Echo
// ---------------------------------------------------------------------------

/// The cognitive-ethical fingerprint of a single user on the Moral Manifold.
///
/// This "Echo" allows the network to generate higher-order hypotheses
/// (meta-ethical and global coordination) based on aggregated collective intuition.
#[derive(Debug, Clone, PartialEq)]
pub struct PersonalEcho {
    /// Unique user identifier.
    pub user_id: u64,
    /// Participation weight (contribution ratio to the network).
    pub participation: f64,
    /// Cognitive coherence score [0.0, 1.0].
    pub coherence: f64,
    /// Ethical alignment on the Moral Manifold [-1.0, 1.0].
    pub ethical_alignment: f64,
    /// Timestamp when this echo was last updated.
    pub updated_at_ms: u64,
    /// Number of contributions by this user.
    pub contribution_count: u32,
    /// Domain specialization vector (compressed as f64 array).
    pub domain_vector: [f64; 8],
}

impl PersonalEcho {
    /// Create a new Personal Echo.
    pub fn new(
        user_id: u64,
        participation: f64,
        coherence: f64,
        ethical_alignment: f64,
        updated_at_ms: u64,
    ) -> Result<Self, ResonanceError> {
        if participation < 0.0 {
            return Err(ResonanceError::NegativeParticipation(participation));
        }
        if !(0.0..=1.0).contains(&coherence) {
            return Err(ResonanceError::CoherenceOutOfRange(coherence));
        }
        if !(-1.0..=1.0).contains(&ethical_alignment) {
            return Err(ResonanceError::AlignmentOutOfRange(ethical_alignment));
        }

        Ok(Self {
            user_id,
            participation,
            coherence,
            ethical_alignment,
            updated_at_ms,
            contribution_count: 0,
            domain_vector: [0.0; 8],
        })
    }

    /// Update echo with new contribution data.
    pub fn record_contribution(
        &mut self,
        coherence_delta: f64,
        alignment_delta: f64,
        timestamp_ms: u64,
        domain_index: usize,
    ) -> Result<(), ResonanceError> {
        self.contribution_count += 1;
        self.updated_at_ms = timestamp_ms;

        // Update coherence with delta (clamped)
        self.coherence = (self.coherence + coherence_delta).clamp(0.0, 1.0);

        // Update alignment with delta (clamped)
        self.ethical_alignment = (self.ethical_alignment + alignment_delta).clamp(-1.0, 1.0);

        // Update domain vector
        if domain_index < 8 {
            self.domain_vector[domain_index] += 1.0 / self.contribution_count as f64;
        }

        Ok(())
    }

    /// Compute the echo's resonance contribution: participation * coherence * alignment.
    pub fn resonance_contribution(&self) -> f64 {
        self.participation * self.coherence * (1.0 + self.ethical_alignment) / 2.0
    }

    /// Normalize domain vector to sum to 1.0.
    pub fn normalized_domain_vector(&self) -> [f64; 8] {
        let sum: f64 = self.domain_vector.iter().sum();
        if sum < 1e-10 {
            return [0.0; 8];
        }
        let mut normalized = [0.0; 8];
        for (i, val) in normalized.iter_mut().enumerate().take(8) {
            *val = self.domain_vector[i] / sum;
        }
        normalized
    }

    /// Compute similarity with another echo (cosine-like on domain vectors).
    pub fn similarity(&self, other: &PersonalEcho) -> f64 {
        let dot: f64 = self
            .domain_vector
            .iter()
            .zip(other.domain_vector.iter())
            .map(|(a, b)| a * b)
            .sum();
        let mag_self: f64 = self.domain_vector.iter().map(|v| v * v).sum::<f64>().sqrt();
        let mag_other: f64 = other
            .domain_vector
            .iter()
            .map(|v| v * v)
            .sum::<f64>()
            .sqrt();
        let denom = mag_self * mag_other;
        if denom < 1e-10 {
            return 0.0;
        }
        (dot / denom).clamp(-1.0, 1.0)
    }
}

impl std::fmt::Display for PersonalEcho {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Echo[user={}, coherence={:.3}, alignment={:.3}, contributions={}]",
            self.user_id, self.coherence, self.ethical_alignment, self.contribution_count
        )
    }
}

// ---------------------------------------------------------------------------
// Universal Resonance Calculator
// ---------------------------------------------------------------------------

/// Aggregates Personal Echoes to compute R_universal(t).
pub struct UniversalResonance {
    /// Registry of all active echoes.
    echoes: Vec<PersonalEcho>,
    /// Global participation normalization factor.
    total_participation: f64,
    /// Current R_universal value.
    current_resonance: f64,
    /// Timestamp of last computation.
    last_computed_at_ms: u64,
    /// Counter for computation calls.
    computation_counter: u64,
}

impl UniversalResonance {
    /// Create a new Universal Resonance calculator.
    pub fn new() -> Self {
        Self {
            echoes: Vec::new(),
            total_participation: 0.0,
            current_resonance: 0.0,
            last_computed_at_ms: 0,
            computation_counter: 0,
        }
    }

    /// Register or update a Personal Echo.
    pub fn register_echo(&mut self, echo: PersonalEcho) {
        // Remove existing echo for this user if present
        if let Some(pos) = self.echoes.iter().position(|e| e.user_id == echo.user_id) {
            let old = &self.echoes[pos];
            self.total_participation -= old.participation;
            self.echoes.remove(pos);
        }

        self.total_participation += echo.participation;
        self.echoes.push(echo);
    }

    /// Remove a user's echo from the registry.
    pub fn unregister_echo(&mut self, user_id: u64) -> Result<PersonalEcho, ResonanceError> {
        let pos = self
            .echoes
            .iter()
            .position(|e| e.user_id == user_id)
            .ok_or(ResonanceError::UserIdNotFound(user_id))?;

        let echo = self.echoes.remove(pos);
        self.total_participation -= echo.participation;
        Ok(echo)
    }

    /// Get a user's echo by ID.
    pub fn get_echo(&self, user_id: u64) -> Result<&PersonalEcho, ResonanceError> {
        self.echoes
            .iter()
            .find(|e| e.user_id == user_id)
            .ok_or(ResonanceError::UserIdNotFound(user_id))
    }

    /// Compute R_universal(t).
    ///
    /// ```text
    /// R_universal(t) = Σ [ p_i * echo_i.coherence * echo_i.ethical_alignment ] / Σ p_i
    /// ```
    pub fn compute(&mut self, timestamp_ms: u64) -> Result<f64, ResonanceError> {
        if self.echoes.is_empty() {
            return Err(ResonanceError::NoEchoesAvailable);
        }

        let weighted_sum: f64 = self.echoes.iter().map(|e| e.resonance_contribution()).sum();

        self.current_resonance = if self.total_participation > 1e-10 {
            weighted_sum / self.total_participation
        } else {
            weighted_sum / self.echoes.len() as f64
        };

        self.last_computed_at_ms = timestamp_ms;
        self.computation_counter += 1;

        Ok(self.current_resonance)
    }

    /// Get current R_universal value.
    pub fn current_resonance(&self) -> f64 {
        self.current_resonance
    }

    /// Get number of registered echoes.
    pub fn echo_count(&self) -> usize {
        self.echoes.len()
    }

    /// Get all echoes.
    pub fn echoes(&self) -> &[PersonalEcho] {
        &self.echoes
    }

    /// Get total participation weight.
    pub fn total_participation(&self) -> f64 {
        self.total_participation
    }

    /// Find the most coherent echo (highest coherence score).
    pub fn most_coherent_echo(&self) -> Option<&PersonalEcho> {
        self.echoes
            .iter()
            .max_by(|a, b| a.coherence.partial_cmp(&b.coherence).unwrap())
    }

    /// Find the most ethically aligned echo (highest alignment).
    pub fn most_aligned_echo(&self) -> Option<&PersonalEcho> {
        self.echoes.iter().max_by(|a, b| {
            a.ethical_alignment
                .partial_cmp(&b.ethical_alignment)
                .unwrap()
        })
    }

    /// Compute average coherence across all echoes.
    pub fn average_coherence(&self) -> f64 {
        if self.echoes.is_empty() {
            return 0.0;
        }
        self.echoes.iter().map(|e| e.coherence).sum::<f64>() / self.echoes.len() as f64
    }

    /// Compute average ethical alignment across all echoes.
    pub fn average_alignment(&self) -> f64 {
        if self.echoes.is_empty() {
            return 0.0;
        }
        self.echoes.iter().map(|e| e.ethical_alignment).sum::<f64>() / self.echoes.len() as f64
    }

    /// Generate a collective meta-ethical hypothesis from aggregated echoes.
    ///
    /// Returns a weighted domain vector representing the collective intuition.
    pub fn collective_hypothesis(&self) -> [f64; 8] {
        if self.echoes.is_empty() {
            return [0.0; 8];
        }

        let mut hypothesis = [0.0; 8];
        let total_weight: f64 = self
            .echoes
            .iter()
            .map(|e| e.participation * e.coherence)
            .sum();

        if total_weight < 1e-10 {
            return [0.0; 8];
        }

        for echo in &self.echoes {
            let weight = echo.participation * echo.coherence / total_weight;
            let normalized = echo.normalized_domain_vector();
            for i in 0..8 {
                hypothesis[i] += weight * normalized[i];
            }
        }

        hypothesis
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        self.echoes.clear();
        self.total_participation = 0.0;
        self.current_resonance = 0.0;
        self.last_computed_at_ms = 0;
        self.computation_counter = 0;
    }
}

impl Default for UniversalResonance {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for UniversalResonance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "R_universal={:.4} echoes={} participation={:.2}",
            self.current_resonance,
            self.echo_count(),
            self.total_participation
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- PersonalEcho ---

    #[test]
    fn test_echo_creation() {
        let echo = PersonalEcho::new(1, 0.5, 0.8, 0.6, 1000).unwrap();
        assert_eq!(echo.user_id, 1);
        assert_eq!(echo.participation, 0.5);
        assert_eq!(echo.coherence, 0.8);
        assert_eq!(echo.ethical_alignment, 0.6);
        assert_eq!(echo.contribution_count, 0);
    }

    #[test]
    fn test_echo_negative_participation() {
        match PersonalEcho::new(1, -0.1, 0.8, 0.6, 1000) {
            Err(ResonanceError::NegativeParticipation(val)) => assert!((val + 0.1).abs() < 1e-10),
            other => panic!("Expected NegativeParticipation, got {:?}", other),
        }
    }

    #[test]
    fn test_echo_coherence_out_of_range() {
        match PersonalEcho::new(1, 0.5, 1.5, 0.6, 1000) {
            Err(ResonanceError::CoherenceOutOfRange(val)) => assert!((val - 1.5).abs() < 1e-10),
            other => panic!("Expected CoherenceOutOfRange, got {:?}", other),
        }
    }

    #[test]
    fn test_echo_alignment_out_of_range() {
        match PersonalEcho::new(1, 0.5, 0.8, 1.5, 1000) {
            Err(ResonanceError::AlignmentOutOfRange(val)) => assert!((val - 1.5).abs() < 1e-10),
            other => panic!("Expected AlignmentOutOfRange, got {:?}", other),
        }
    }

    #[test]
    fn test_echo_contribution() {
        let mut echo = PersonalEcho::new(1, 0.5, 0.8, 0.6, 1000).unwrap();
        echo.record_contribution(0.05, 0.02, 2000, 0).unwrap();
        assert_eq!(echo.contribution_count, 1);
        assert!((echo.coherence - 0.85).abs() < 1e-10);
        assert!((echo.ethical_alignment - 0.62).abs() < 1e-10);
        assert_eq!(echo.updated_at_ms, 2000);
    }

    #[test]
    fn test_echo_contribution_clamping() {
        let mut echo = PersonalEcho::new(1, 0.5, 0.95, 0.9, 1000).unwrap();
        echo.record_contribution(0.1, 0.1, 2000, 0).unwrap();
        assert_eq!(echo.coherence, 1.0); // Clamped
        assert_eq!(echo.ethical_alignment, 1.0); // Clamped
    }

    #[test]
    fn test_echo_resonance_contribution() {
        let echo = PersonalEcho::new(1, 0.5, 0.8, 0.0, 1000).unwrap();
        // contribution = 0.5 * 0.8 * (1.0 + 0.0) / 2.0 = 0.2
        assert!((echo.resonance_contribution() - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_echo_normalized_domain_vector() {
        let mut echo = PersonalEcho::new(1, 0.5, 0.8, 0.6, 1000).unwrap();
        echo.record_contribution(0.0, 0.0, 2000, 0).unwrap();
        echo.record_contribution(0.0, 0.0, 3000, 2).unwrap();

        let normalized = echo.normalized_domain_vector();
        let sum: f64 = normalized.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
        assert!(normalized[0] > 0.0);
        assert!(normalized[2] > 0.0);
    }

    #[test]
    fn test_echo_similarity_identical() {
        let echo1 = PersonalEcho::new(1, 0.5, 0.8, 0.6, 1000).unwrap();
        let echo2 = PersonalEcho::new(2, 0.5, 0.8, 0.6, 1000).unwrap();
        // Both have zero domain vectors, so similarity is 0.0
        assert!((echo1.similarity(&echo2) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_echo_similarity_with_domains() {
        let mut echo1 = PersonalEcho::new(1, 0.5, 0.8, 0.6, 1000).unwrap();
        let mut echo2 = PersonalEcho::new(2, 0.5, 0.8, 0.6, 1000).unwrap();

        echo1.record_contribution(0.0, 0.0, 2000, 0).unwrap();
        echo2.record_contribution(0.0, 0.0, 2000, 0).unwrap();

        let sim = echo1.similarity(&echo2);
        assert!((sim - 1.0).abs() < 1e-10); // Same domain = perfect similarity
    }

    #[test]
    fn test_echo_display() {
        let echo = PersonalEcho::new(1, 0.5, 0.8, 0.6, 1000).unwrap();
        let display = format!("{}", echo);
        assert!(display.contains("Echo["));
        assert!(display.contains("user="));
    }

    // --- UniversalResonance ---

    #[test]
    fn test_resonance_creation() {
        let r = UniversalResonance::new();
        assert_eq!(r.echo_count(), 0);
        assert_eq!(r.current_resonance(), 0.0);
    }

    #[test]
    fn test_register_echo() {
        let mut r = UniversalResonance::new();
        let echo = PersonalEcho::new(1, 0.5, 0.8, 0.6, 1000).unwrap();
        r.register_echo(echo);
        assert_eq!(r.echo_count(), 1);
        assert!((r.total_participation() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_register_echo_update() {
        let mut r = UniversalResonance::new();
        let echo1 = PersonalEcho::new(1, 0.5, 0.8, 0.6, 1000).unwrap();
        r.register_echo(echo1);
        let echo2 = PersonalEcho::new(1, 0.8, 0.9, 0.7, 2000).unwrap();
        r.register_echo(echo2);
        assert_eq!(r.echo_count(), 1); // Same user, replaced
        assert!((r.total_participation() - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_unregister_echo() {
        let mut r = UniversalResonance::new();
        let echo = PersonalEcho::new(1, 0.5, 0.8, 0.6, 1000).unwrap();
        r.register_echo(echo);
        let removed = r.unregister_echo(1).unwrap();
        assert_eq!(removed.user_id, 1);
        assert_eq!(r.echo_count(), 0);
    }

    #[test]
    fn test_unregister_echo_not_found() {
        let mut r = UniversalResonance::new();
        match r.unregister_echo(999) {
            Err(ResonanceError::UserIdNotFound(999)) => {}
            other => panic!("Expected UserIdNotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_get_echo() {
        let mut r = UniversalResonance::new();
        let echo = PersonalEcho::new(1, 0.5, 0.8, 0.6, 1000).unwrap();
        r.register_echo(echo);
        let found = r.get_echo(1).unwrap();
        assert_eq!(found.user_id, 1);
    }

    #[test]
    fn test_get_echo_not_found() {
        let r = UniversalResonance::new();
        match r.get_echo(999) {
            Err(ResonanceError::UserIdNotFound(999)) => {}
            other => panic!("Expected UserIdNotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_compute_resonance() {
        let mut r = UniversalResonance::new();
        let echo = PersonalEcho::new(1, 1.0, 0.8, 0.0, 1000).unwrap();
        r.register_echo(echo);

        let res = r.compute(2000).unwrap();
        // contribution = 1.0 * 0.8 * (1.0 + 0.0) / 2.0 = 0.4
        // R = 0.4 / 1.0 = 0.4
        assert!((res - 0.4).abs() < 1e-10);
    }

    #[test]
    fn test_compute_resonance_multiple_echoes() {
        let mut r = UniversalResonance::new();
        r.register_echo(PersonalEcho::new(1, 1.0, 0.8, 0.0, 1000).unwrap());
        r.register_echo(PersonalEcho::new(2, 1.0, 0.6, 0.0, 1000).unwrap());

        let res = r.compute(2000).unwrap();
        // echo1: 1.0 * 0.8 * 0.5 = 0.4
        // echo2: 1.0 * 0.6 * 0.5 = 0.3
        // R = (0.4 + 0.3) / 2.0 = 0.35
        assert!((res - 0.35).abs() < 1e-10);
    }

    #[test]
    fn test_compute_no_echoes() {
        let mut r = UniversalResonance::new();
        match r.compute(1000) {
            Err(ResonanceError::NoEchoesAvailable) => {}
            other => panic!("Expected NoEchoesAvailable, got {:?}", other),
        }
    }

    #[test]
    fn test_average_coherence() {
        let mut r = UniversalResonance::new();
        r.register_echo(PersonalEcho::new(1, 1.0, 0.8, 0.0, 1000).unwrap());
        r.register_echo(PersonalEcho::new(2, 1.0, 0.6, 0.0, 1000).unwrap());
        assert!((r.average_coherence() - 0.7).abs() < 1e-10);
    }

    #[test]
    fn test_average_alignment() {
        let mut r = UniversalResonance::new();
        r.register_echo(PersonalEcho::new(1, 1.0, 0.8, 0.4, 1000).unwrap());
        r.register_echo(PersonalEcho::new(2, 1.0, 0.6, -0.4, 1000).unwrap());
        assert!((r.average_alignment() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_most_coherent_echo() {
        let mut r = UniversalResonance::new();
        r.register_echo(PersonalEcho::new(1, 1.0, 0.6, 0.0, 1000).unwrap());
        r.register_echo(PersonalEcho::new(2, 1.0, 0.9, 0.0, 1000).unwrap());
        let best = r.most_coherent_echo().unwrap();
        assert_eq!(best.user_id, 2);
    }

    #[test]
    fn test_most_aligned_echo() {
        let mut r = UniversalResonance::new();
        r.register_echo(PersonalEcho::new(1, 1.0, 0.8, 0.3, 1000).unwrap());
        r.register_echo(PersonalEcho::new(2, 1.0, 0.8, 0.7, 1000).unwrap());
        let best = r.most_aligned_echo().unwrap();
        assert_eq!(best.user_id, 2);
    }

    #[test]
    fn test_collective_hypothesis() {
        let mut r = UniversalResonance::new();
        let mut echo1 = PersonalEcho::new(1, 1.0, 0.8, 0.0, 1000).unwrap();
        echo1.record_contribution(0.0, 0.0, 2000, 0).unwrap();
        r.register_echo(echo1);

        let hypothesis = r.collective_hypothesis();
        assert!(hypothesis[0] > 0.0);
        let sum: f64 = hypothesis.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_collective_hypothesis_empty() {
        let r = UniversalResonance::new();
        let hypothesis = r.collective_hypothesis();
        assert_eq!(hypothesis, [0.0; 8]);
    }

    #[test]
    fn test_reset() {
        let mut r = UniversalResonance::new();
        r.register_echo(PersonalEcho::new(1, 1.0, 0.8, 0.0, 1000).unwrap());
        r.compute(2000).unwrap();
        r.reset();
        assert_eq!(r.echo_count(), 0);
        assert_eq!(r.current_resonance(), 0.0);
        assert_eq!(r.total_participation(), 0.0);
    }

    #[test]
    fn test_default_impl() {
        let r = UniversalResonance::default();
        assert_eq!(r.echo_count(), 0);
    }

    #[test]
    fn test_resonance_display() {
        let r = UniversalResonance::new();
        let display = format!("{}", r);
        assert!(display.contains("R_universal="));
    }

    #[test]
    fn test_error_display() {
        let err = ResonanceError::NegativeParticipation(-0.5);
        let msg = format!("{}", err);
        assert!(msg.contains("non-negative"));
    }

    #[test]
    fn test_error_display_no_echoes() {
        let err = ResonanceError::NoEchoesAvailable;
        let msg = format!("{}", err);
        assert!(msg.contains("No echoes"));
    }

    #[test]
    fn test_error_display_user_not_found() {
        let err = ResonanceError::UserIdNotFound(42);
        let msg = format!("{}", err);
        assert!(msg.contains("42"));
    }

    // --- Integration Tests ---

    #[test]
    fn test_full_resonance_workflow() {
        let mut r = UniversalResonance::new();

        // Register 5 users with varying coherence and alignment
        for i in 0..5 {
            let coherence = 0.5 + i as f64 * 0.1;
            let alignment = -0.4 + i as f64 * 0.2;
            let echo = PersonalEcho::new(i, 1.0, coherence, alignment, 1000).unwrap();
            r.register_echo(echo);
        }

        assert_eq!(r.echo_count(), 5);

        // Compute resonance
        let res = r.compute(2000).unwrap();
        assert!(res > 0.0);
        assert!(res <= 1.0);

        // Verify averages
        let avg_coh = r.average_coherence();
        assert!((avg_coh - 0.7).abs() < 1e-10);

        let avg_align = r.average_alignment();
        assert!((avg_align - 0.0).abs() < 1e-10);

        // Get collective hypothesis
        let hypothesis = r.collective_hypothesis();
        let sum: f64 = hypothesis.iter().sum();
        assert!((sum - 0.0).abs() < 1e-10); // No domain contributions yet
    }

    #[test]
    fn test_resonance_with_contributions() {
        let mut r = UniversalResonance::new();

        let mut echo1 = PersonalEcho::new(1, 1.0, 0.8, 0.5, 1000).unwrap();
        echo1.record_contribution(0.05, 0.02, 2000, 0).unwrap();
        echo1.record_contribution(0.03, 0.01, 3000, 3).unwrap();
        r.register_echo(echo1);

        let res = r.compute(4000).unwrap();
        assert!(res > 0.0);

        let found = r.get_echo(1).unwrap();
        assert_eq!(found.contribution_count, 2);
    }

    #[test]
    fn test_echoes_slice_access() {
        let mut r = UniversalResonance::new();
        r.register_echo(PersonalEcho::new(1, 1.0, 0.8, 0.0, 1000).unwrap());
        r.register_echo(PersonalEcho::new(2, 1.0, 0.6, 0.0, 1000).unwrap());

        let echoes = r.echoes();
        assert_eq!(echoes.len(), 2);
        assert_eq!(echoes[0].user_id, 1);
        assert_eq!(echoes[1].user_id, 2);
    }
}
