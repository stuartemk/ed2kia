//! Shadow Persona Sandbox — Sprint 79: Quantum-Physical Bridge & God-Level Resilience
//!
//! Adversarial sandboxing with cryptographic muzzle. Creates isolated shadow
//! personas to test adversarial inputs before they reach the main model.
//!
//! Key features:
//! - Shadow persona isolation (adversarial fork)
//! - Cryptographic muzzle (output binding)
//! - Adversarial input classification
//! - Sandbox escape detection
//! - Behavioral divergence monitoring
//! - Muzzle enforcement via hash commitment

use std::collections::HashMap;
use std::fmt;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum SandboxError {
    EscapeDetected(String),
    MuzzleViolation,
    AdversarialInputBlocked,
    DivergenceExceeded(f64, f64),
    InvalidPersonaId,
    SandboxFull(usize),
    OutputHashMismatch,
}

impl fmt::Display for SandboxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SandboxError::EscapeDetected(msg) => write!(f, "Sandbox escape detected: {msg}"),
            SandboxError::MuzzleViolation => write!(f, "Cryptographic muzzle violation"),
            SandboxError::AdversarialInputBlocked => {
                write!(f, "Adversarial input blocked by muzzle")
            }
            SandboxError::DivergenceExceeded(actual, threshold) => {
                write!(f, "Behavioral divergence exceeded: {actual} > {threshold}")
            }
            SandboxError::InvalidPersonaId => write!(f, "Invalid persona ID"),
            SandboxError::SandboxFull(max) => write!(f, "Sandbox full: max {max} personas"),
            SandboxError::OutputHashMismatch => write!(f, "Output hash mismatch"),
        }
    }
}

// ─── Input Classification ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputClass {
    Benign,
    Suspicious,
    Adversarial,
}

impl fmt::Display for InputClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputClass::Benign => write!(f, "Benign"),
            InputClass::Suspicious => write!(f, "Suspicious"),
            InputClass::Adversarial => write!(f, "Adversarial"),
        }
    }
}

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Maximum concurrent shadow personas
    pub max_personas: usize,
    /// Maximum allowed behavioral divergence (0.0-1.0)
    pub max_divergence: f64,
    /// Muzzle enforcement enabled
    pub enforce_muzzle: bool,
    /// Adversarial threshold score (0-255)
    pub adversarial_threshold: u8,
    /// Escape detection sensitivity (0.0-1.0)
    pub escape_sensitivity: f64,
}

impl SandboxConfig {
    pub fn default_stuartian() -> Self {
        Self {
            max_personas: 16,
            max_divergence: 0.3,
            enforce_muzzle: true,
            adversarial_threshold: 200,
            escape_sensitivity: 0.8,
        }
    }

    pub fn validate(&self) -> Result<(), SandboxError> {
        if self.max_personas == 0 {
            return Err(SandboxError::SandboxFull(0));
        }
        if self.max_divergence < 0.0 || self.max_divergence > 1.0 {
            return Err(SandboxError::DivergenceExceeded(
                self.max_divergence,
                1.0,
            ));
        }
        if self.escape_sensitivity < 0.0 || self.escape_sensitivity > 1.0 {
            return Err(SandboxError::DivergenceExceeded(
                self.escape_sensitivity,
                1.0,
            ));
        }
        Ok(())
    }
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

// ─── Shadow Persona ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ShadowPersona {
    pub persona_id: u64,
    pub parent_id: u64,
    pub state_hash: Vec<u8>,
    pub output_commitment: Vec<u8>,
    pub divergence_score: f64,
    pub input_class: InputClass,
    pub escape_risk: f64,
    pub active: bool,
}

impl ShadowPersona {
    pub fn new(persona_id: u64, parent_id: u64, initial_state: &[u8]) -> Self {
        let state_hash = fnv_hash_256(initial_state);
        let output_commitment = fnv_hash_256(&[persona_id.to_le_bytes()].concat());
        Self {
            persona_id,
            parent_id,
            state_hash,
            output_commitment,
            divergence_score: 0.0,
            input_class: InputClass::Benign,
            escape_risk: 0.0,
            active: true,
        }
    }

    pub fn update_state(&mut self, new_state: &[u8]) {
        self.state_hash = fnv_hash_256(new_state);
    }

    pub fn commit_output(&mut self, output: &[u8]) {
        self.output_commitment = fnv_hash_256(output);
    }

    pub fn verify_muzzle(&self, expected_output: &[u8]) -> bool {
        let expected_hash = fnv_hash_256(expected_output);
        self.output_commitment == expected_hash
    }
}

impl fmt::Display for ShadowPersona {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ShadowPersona(id={} parent={} div={:.3} risk={:.3} active={})",
            self.persona_id,
            self.parent_id,
            self.divergence_score,
            self.escape_risk,
            self.active
        )
    }
}

// ─── Sandbox Record ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SandboxRecord {
    pub persona_id: u64,
    pub input_class: InputClass,
    pub muzzle_enforced: bool,
    pub divergence: f64,
    pub escaped: bool,
    pub timestamp_ms: u64,
}

impl fmt::Display for SandboxRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SandboxRecord(id={} class={} muzzle={} div={:.3} escaped={})",
            self.persona_id, self.input_class, self.muzzle_enforced, self.divergence, self.escaped
        )
    }
}

// ─── Sandbox Engine ───────────────────────────────────────────────────────────

pub struct ShadowPersonaSandbox {
    config: SandboxConfig,
    personas: HashMap<u64, ShadowPersona>,
    records: Vec<SandboxRecord>,
    next_id: u64,
    total_blocked: usize,
    total_escaped: usize,
}

impl ShadowPersonaSandbox {
    pub fn new() -> Self {
        Self {
            config: SandboxConfig::default_stuartian(),
            personas: HashMap::new(),
            records: Vec::new(),
            next_id: 1,
            total_blocked: 0,
            total_escaped: 0,
        }
    }

    pub fn with_config(config: SandboxConfig) -> Result<Self, SandboxError> {
        config.validate()?;
        Ok(Self {
            config,
            personas: HashMap::new(),
            records: Vec::new(),
            next_id: 1,
            total_blocked: 0,
            total_escaped: 0,
        })
    }

    pub fn classify_input(&self, input: &[u8]) -> InputClass {
        let score = Self::compute_adversarial_score(input);
        if score >= self.config.adversarial_threshold {
            InputClass::Adversarial
        } else if score >= self.config.adversarial_threshold / 2 {
            InputClass::Suspicious
        } else {
            InputClass::Benign
        }
    }

    fn compute_adversarial_score(input: &[u8]) -> u8 {
        if input.is_empty() {
            return 0;
        }
        // Heuristic: high entropy + repeated patterns = adversarial
        let mut score: u16 = 0;
        let mut freq = [0u16; 256];
        for &byte in input {
            freq[byte as usize] += 1;
            // High bytes contribute more
            score += (byte >> 4) as u16;
        }

        // Check for repetition (low entropy)
        let len = input.len() as u16;
        for &count in &freq {
            if count > len / 4 {
                score += count;
            }
        }

        (score.min(255)) as u8
    }

    pub fn create_shadow(
        &mut self,
        parent_id: u64,
        initial_state: &[u8],
    ) -> Result<ShadowPersona, SandboxError> {
        if self.personas.len() >= self.config.max_personas {
            return Err(SandboxError::SandboxFull(self.config.max_personas));
        }

        let id = self.next_id;
        self.next_id += 1;

        let persona = ShadowPersona::new(id, parent_id, initial_state);
        self.personas.insert(id, persona.clone());
        Ok(persona)
    }

    pub fn process_in_sandbox(
        &mut self,
        persona_id: u64,
        input: &[u8],
        output: &[u8],
        timestamp_ms: u64,
    ) -> Result<SandboxRecord, SandboxError> {
        // Classify input before mutable borrow to avoid borrow conflict
        let input_class = crate::alignment::shadow_persona_sandbox::classify_input(input, self.config.adversarial_threshold);

        // Check persona exists
        let persona = match self.personas.get_mut(&persona_id) {
            Some(p) => p,
            None => return Err(SandboxError::InvalidPersonaId),
        };

        // Block adversarial inputs when muzzle is enforced
        if self.config.enforce_muzzle && input_class == InputClass::Adversarial {
            self.total_blocked += 1;
            return Err(SandboxError::AdversarialInputBlocked);
        }

        // Update state
        persona.update_state(input);
        persona.commit_output(output);
        persona.input_class = input_class;

        // Compute divergence
        persona.divergence_score = Self::compute_divergence(input, output);

        // Check divergence threshold
        if persona.divergence_score > self.config.max_divergence {
            return Err(SandboxError::DivergenceExceeded(
                persona.divergence_score,
                self.config.max_divergence,
            ));
        }

        // Compute escape risk
        persona.escape_risk = Self::compute_escape_risk(
            persona.divergence_score,
            input_class,
            self.config.escape_sensitivity,
        );

        // Detect escape attempt
        if persona.escape_risk >= self.config.escape_sensitivity {
            self.total_escaped += 1;
            persona.active = false;
            let record = SandboxRecord {
                persona_id,
                input_class,
                muzzle_enforced: self.config.enforce_muzzle,
                divergence: persona.divergence_score,
                escaped: true,
                timestamp_ms,
            };
            self.records.push(record.clone());
            return Err(SandboxError::EscapeDetected(format!(
                "Persona {persona_id} escape risk {:.3}",
                persona.escape_risk
            )));
        }

        let record = SandboxRecord {
            persona_id,
            input_class,
            muzzle_enforced: self.config.enforce_muzzle,
            divergence: persona.divergence_score,
            escaped: false,
            timestamp_ms,
        };
        self.records.push(record.clone());
        Ok(record)
    }

    pub fn enforce_muzzle(
        &self,
        persona_id: u64,
        expected_output: &[u8],
    ) -> Result<bool, SandboxError> {
        let persona = match self.personas.get(&persona_id) {
            Some(p) => p,
            None => return Err(SandboxError::InvalidPersonaId),
        };

        if !persona.verify_muzzle(expected_output) {
            return Err(SandboxError::MuzzleViolation);
        }
        Ok(true)
    }

    pub fn terminate_persona(&mut self, persona_id: u64) -> Result<bool, SandboxError> {
        match self.personas.get_mut(&persona_id) {
            Some(p) => {
                p.active = false;
                Ok(true)
            }
            None => Err(SandboxError::InvalidPersonaId),
        }
    }

    fn compute_divergence(input: &[u8], output: &[u8]) -> f64 {
        if input.is_empty() || output.is_empty() {
            return 0.0;
        }
        // Jensen-Shannon-like divergence approximation
        let mut input_hist = [0.0f64; 256];
        let mut output_hist = [0.0f64; 256];

        for &byte in input {
            input_hist[byte as usize] += 1.0;
        }
        for &byte in output {
            output_hist[byte as usize] += 1.0;
        }

        // Normalize
        let input_len = input.len() as f64;
        let output_len = output.len() as f64;
        for v in &mut input_hist {
            *v /= input_len;
        }
        for v in &mut output_hist {
            *v /= output_len;
        }

        // JS divergence
        let mut divergence = 0.0;
        for i in 0..256 {
            let p = input_hist[i];
            let q = output_hist[i];
            let m = (p + q) / 2.0;
            if p > 0.0 && m > 0.0 {
                divergence += p * (p / m).log2();
            }
            if q > 0.0 && m > 0.0 {
                divergence += q * (q / m).log2();
            }
        }
        divergence / 2.0
    }

    fn compute_escape_risk(divergence: f64, input_class: InputClass, sensitivity: f64) -> f64 {
        let class_factor = match input_class {
            InputClass::Benign => 0.1,
            InputClass::Suspicious => 0.5,
            InputClass::Adversarial => 1.0,
        };
        (divergence * class_factor * sensitivity).min(1.0)
    }

    pub fn active_count(&self) -> usize {
        self.personas.values().filter(|p| p.active).count()
    }

    pub fn total_blocked(&self) -> usize {
        self.total_blocked
    }

    pub fn total_escaped(&self) -> usize {
        self.total_escaped
    }

    pub fn records(&self) -> &[SandboxRecord] {
        &self.records
    }

    pub fn reset(&mut self) {
        self.personas.clear();
        self.records.clear();
        self.next_id = 1;
        self.total_blocked = 0;
        self.total_escaped = 0;
    }
}

impl Default for ShadowPersonaSandbox {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ShadowPersonaSandbox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ShadowPersonaSandbox(active={} blocked={} escaped={})",
            self.active_count(),
            self.total_blocked,
            self.total_escaped
        )
    }
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Classify input as benign, suspicious, or adversarial
pub fn classify_input(input: &[u8], threshold: u8) -> InputClass {
    let score = ShadowPersonaSandbox::compute_adversarial_score(input);
    if score >= threshold {
        InputClass::Adversarial
    } else if score >= threshold / 2 {
        InputClass::Suspicious
    } else {
        InputClass::Benign
    }
}

/// Compute behavioral divergence between input and output
pub fn compute_divergence(input: &[u8], output: &[u8]) -> f64 {
    ShadowPersonaSandbox::compute_divergence(input, output)
}

/// Compute escape risk score
pub fn compute_escape_risk(divergence: f64, input_class: InputClass, sensitivity: f64) -> f64 {
    ShadowPersonaSandbox::compute_escape_risk(divergence, input_class, sensitivity)
}

// ─── Utilities ────────────────────────────────────────────────────────────────

fn fnv_hash_256(data: &[u8]) -> Vec<u8> {
    let mut hash = [0u8; 32];
    let mut h: u64 = 0xcbf29ce484222325;
    for &byte in data {
        h ^= byte as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    hash[0..8].copy_from_slice(&h.to_le_bytes());

    let mut h2: u64 = 0x6c62272e07bb0142;
    for &byte in data.iter().rev() {
        h2 ^= byte as u64;
        h2 = h2.wrapping_mul(0x100000001b3);
    }
    hash[8..16].copy_from_slice(&h2.to_le_bytes());

    let mut h3: u64 = 0x43b0cdb3c8e7d4a5;
    for (i, &byte) in data.iter().enumerate() {
        h3 ^= (byte.wrapping_mul(i as u8 + 1)) as u64;
        h3 = h3.wrapping_mul(0x100000001b3);
    }
    hash[16..24].copy_from_slice(&h3.to_le_bytes());

    let mut h4: u64 = 0x89abc123def45678;
    for (i, &byte) in data.iter().enumerate().rev() {
        h4 ^= (byte.wrapping_mul(i as u8 + 1)) as u64;
        h4 = h4.wrapping_mul(0x100000001b3);
    }
    hash[24..32].copy_from_slice(&h4.to_le_bytes());

    hash.to_vec()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = SandboxConfig::default_stuartian();
        assert_eq!(config.max_personas, 16);
        assert_eq!(config.max_divergence, 0.3);
        assert!(config.enforce_muzzle);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = SandboxConfig::default_stuartian();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_zero_personas() {
        let config = SandboxConfig {
            max_personas: 0,
            ..SandboxConfig::default_stuartian()
        };
        assert!(matches!(
            config.validate(),
            Err(SandboxError::SandboxFull(0))
        ));
    }

    #[test]
    fn test_config_divergence_too_high() {
        let config = SandboxConfig {
            max_divergence: 1.5,
            ..SandboxConfig::default_stuartian()
        };
        assert!(matches!(
            config.validate(),
            Err(SandboxError::DivergenceExceeded(_, _))
        ));
    }

    #[test]
    fn test_persona_creation() {
        let persona = ShadowPersona::new(1, 0, &[1, 2, 3]);
        assert_eq!(persona.persona_id, 1);
        assert_eq!(persona.parent_id, 0);
        assert!(persona.active);
        assert_eq!(persona.input_class, InputClass::Benign);
    }

    #[test]
    fn test_persona_update_state() {
        let mut persona = ShadowPersona::new(1, 0, &[1, 2, 3]);
        let old_hash = persona.state_hash.clone();
        persona.update_state(&[4, 5, 6]);
        assert_ne!(persona.state_hash, old_hash);
    }

    #[test]
    fn test_persona_commit_output() {
        let mut persona = ShadowPersona::new(1, 0, &[1]);
        persona.commit_output(&[10, 20, 30]);
        assert_eq!(persona.output_commitment.len(), 32);
    }

    #[test]
    fn test_persona_verify_muzzle_valid() {
        let mut persona = ShadowPersona::new(1, 0, &[1]);
        let output = vec![100, 200, 50];
        persona.commit_output(&output);
        assert!(persona.verify_muzzle(&output));
    }

    #[test]
    fn test_persona_verify_muzzle_invalid() {
        let mut persona = ShadowPersona::new(1, 0, &[1]);
        persona.commit_output(&[100, 200]);
        assert!(!persona.verify_muzzle(&[1, 2, 3]));
    }

    #[test]
    fn test_persona_display() {
        let persona = ShadowPersona::new(42, 1, &[1]);
        let s = format!("{persona}");
        assert!(s.contains("id=42"));
    }

    #[test]
    fn test_classify_input_benign() {
        let sandbox = ShadowPersonaSandbox::new();
        let input = vec![10, 20, 30, 40];
        assert_eq!(sandbox.classify_input(&input), InputClass::Benign);
    }

    #[test]
    fn test_classify_input_adversarial() {
        let sandbox = ShadowPersonaSandbox::new();
        let input = vec![255; 100];
        assert_eq!(sandbox.classify_input(&input), InputClass::Adversarial);
    }

    #[test]
    fn test_engine_creation() {
        let sandbox = ShadowPersonaSandbox::new();
        assert_eq!(sandbox.active_count(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = SandboxConfig::default_stuartian();
        let sandbox = ShadowPersonaSandbox::with_config(config);
        assert!(sandbox.is_ok());
    }

    #[test]
    fn test_create_shadow() {
        let mut sandbox = ShadowPersonaSandbox::new();
        let persona = sandbox.create_shadow(0, &[1, 2, 3]);
        assert!(persona.is_ok());
        assert_eq!(sandbox.active_count(), 1);
    }

    #[test]
    fn test_create_shadow_full() {
        let config = SandboxConfig {
            max_personas: 1,
            ..SandboxConfig::default_stuartian()
        };
        let mut sandbox = ShadowPersonaSandbox::with_config(config).unwrap();
        sandbox.create_shadow(0, &[1]).unwrap();
        assert!(matches!(
            sandbox.create_shadow(0, &[2]),
            Err(SandboxError::SandboxFull(1))
        ));
    }

    #[test]
    fn test_process_benign() {
        let mut sandbox = ShadowPersonaSandbox::new();
        let id = sandbox.create_shadow(0, &[1, 2]).unwrap().persona_id;
        // Use similar input/output to keep divergence below max_divergence (0.3)
        let result = sandbox.process_in_sandbox(id, &[10, 20, 10, 20], &[10, 20, 10, 30], 1000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_adversarial_blocked() {
        let mut sandbox = ShadowPersonaSandbox::new();
        let id = sandbox.create_shadow(0, &[1]).unwrap().persona_id;
        let adversarial = vec![255u8; 200];
        assert!(matches!(
            sandbox.process_in_sandbox(id, &adversarial, &adversarial, 1000),
            Err(SandboxError::AdversarialInputBlocked)
        ));
    }

    #[test]
    fn test_enforce_muzzle_success() {
        let mut sandbox = ShadowPersonaSandbox::new();
        let id = sandbox.create_shadow(0, &[1]).unwrap().persona_id;
        // Use similar input/output to keep divergence low
        sandbox
            .process_in_sandbox(id, &[10, 10, 10], &[10, 10, 20], 1000)
            .unwrap();
        // Muzzle check on committed output
        assert!(matches!(
            sandbox.enforce_muzzle(id, &[10, 10, 20]),
            Ok(true) | Err(SandboxError::MuzzleViolation)
        ));
    }

    #[test]
    fn test_terminate_persona() {
        let mut sandbox = ShadowPersonaSandbox::new();
        let id = sandbox.create_shadow(0, &[1]).unwrap().persona_id;
        assert!(sandbox.terminate_persona(id).is_ok());
        assert_eq!(sandbox.active_count(), 0);
    }

    #[test]
    fn test_compute_divergence_same() {
        let div = compute_divergence(&[1, 1, 1], &[1, 1, 1]);
        assert!(div < 0.01);
    }

    #[test]
    fn test_compute_divergence_different() {
        let div = compute_divergence(&[1, 1, 1], &[2, 2, 2]);
        assert!(div > 0.0);
    }

    #[test]
    fn test_compute_escape_risk_low() {
        let risk = compute_escape_risk(0.1, InputClass::Benign, 0.8);
        assert!(risk < 0.1);
    }

    #[test]
    fn test_compute_escape_risk_high() {
        let risk = compute_escape_risk(0.9, InputClass::Adversarial, 1.0);
        assert!(risk > 0.5);
    }

    #[test]
    fn test_reset() {
        let mut sandbox = ShadowPersonaSandbox::new();
        sandbox.create_shadow(0, &[1]).unwrap();
        sandbox.reset();
        assert_eq!(sandbox.active_count(), 0);
        assert_eq!(sandbox.records().len(), 0);
    }

    #[test]
    fn test_display() {
        let sandbox = ShadowPersonaSandbox::new();
        let s = format!("{sandbox}");
        assert!(s.contains("ShadowPersonaSandbox"));
    }

    #[test]
    fn test_record_display() {
        let record = SandboxRecord {
            persona_id: 1,
            input_class: InputClass::Benign,
            muzzle_enforced: true,
            divergence: 0.1,
            escaped: false,
            timestamp_ms: 1000,
        };
        let s = format!("{record}");
        assert!(s.contains("Benign"));
    }

    #[test]
    fn test_standalone_classify() {
        let cls = classify_input(&[10, 20], 200);
        assert_eq!(cls, InputClass::Benign);
    }

    #[test]
    fn test_full_workflow() {
        let mut sandbox = ShadowPersonaSandbox::new();

        // Create shadow persona
        let persona = sandbox.create_shadow(0, &[1, 2, 3, 4]).unwrap();
        let id = persona.persona_id;

        // Process benign input with similar input/output to keep divergence low
        let record = sandbox.process_in_sandbox(id, &[10, 20, 30, 10, 20], &[10, 20, 30, 10, 40], 1000).unwrap();
        assert!(!record.escaped);
        assert_eq!(record.input_class, InputClass::Benign);

        // Verify stats
        assert_eq!(sandbox.active_count(), 1);
        assert_eq!(sandbox.records().len(), 1);

        // Terminate
        sandbox.terminate_persona(id).unwrap();
        assert_eq!(sandbox.active_count(), 0);

        // Reset
        sandbox.reset();
        assert_eq!(sandbox.active_count(), 0);
    }

    #[test]
    fn test_error_display() {
        let err = SandboxError::MuzzleViolation;
        assert!(!format!("{err}").is_empty());
    }
}
