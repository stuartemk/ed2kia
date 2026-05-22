//! Governance Proposal System - Propuestas P2P con firma criptográfica Ed25519
//!
//! Sistema ligero de gobernanza sin blockchain. Cada propuesta es firmada
//! con Ed25519, almacenada localmente en redb, y propagada vía GossipSub.

use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tracing::{info, warn};
use uuid::Uuid;

/// Error del sistema de propuestas
#[derive(Debug, Error)]
pub enum ProposalError {
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),
    #[error("Proposal expired: created={created}, expiry={expiry}")]
    Expired { created: u64, expiry: u64 },
    #[error("Invalid proposal format: {0}")]
    InvalidFormat(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Key generation error: {0}")]
    KeyGeneration(String),
}

/// Estado de la propuesta
// FIX: trait bound - ProposalState needs Eq and Hash for HashMap<ProposalState, usize> usage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ProposalState {
    /// Recién creada, esperando votación
    Proposed,
    /// En período de votación (time-lock activo)
    Voting,
    /// Aprobada por quórum, lista para ejecución
    Approved,
    /// Rechazada por no alcanzar quórum
    Rejected,
    /// Ejecutada exitosamente
    Executed,
    /// Archivada (expirada o cancelada)
    Archived,
}

impl std::fmt::Display for ProposalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProposalState::Proposed => write!(f, "Proposed"),
            ProposalState::Voting => write!(f, "Voting"),
            ProposalState::Approved => write!(f, "Approved"),
            ProposalState::Rejected => write!(f, "Rejected"),
            ProposalState::Executed => write!(f, "Executed"),
            ProposalState::Archived => write!(f, "Archived"),
        }
    }
}

/// Tipo de propuesta
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalType {
    /// Actualización de parámetros de red
    NetworkParam,
    /// Actualización de modelo SAE
    ModelUpdate,
    /// Cambio de política de reputación
    ReputationPolicy,
    /// Propuesta de seguridad
    Security,
    /// Propuesta de gobernanza (meta-governance)
    Governance,
    /// Propuesta de ecosistema (integración externa)
    Ecosystem,
    /// Propuesta custom
    Custom,
}

impl std::fmt::Display for ProposalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProposalType::NetworkParam => write!(f, "NetworkParam"),
            ProposalType::ModelUpdate => write!(f, "ModelUpdate"),
            ProposalType::ReputationPolicy => write!(f, "ReputationPolicy"),
            ProposalType::Security => write!(f, "Security"),
            ProposalType::Governance => write!(f, "Governance"),
            ProposalType::Ecosystem => write!(f, "Ecosystem"),
            ProposalType::Custom => write!(f, "Custom"),
        }
    }
}

/// Propuesta de gobernanza firmada criptográficamente
// FIX: trait bound - VerifyingKey and Signature don't implement Serialize/Deserialize
// Store as hex strings and reconstruct during deserialization
#[derive(Debug, Clone, Serialize)]
pub struct Proposal {
    /// ID único de la propuesta
    pub id: Uuid,
    /// Tipo de propuesta
    pub proposal_type: ProposalType,
    /// Clave pública del autor (Ed25519) - skipped during serialization
    #[serde(skip)]
    pub author: VerifyingKey,
    /// Clave pública como hex string (for serialization)
    pub author_hex: String,
    /// Título de la propuesta
    pub title: String,
    /// Descripción/payload de la propuesta
    pub payload: String,
    /// Timestamp de creación (epoch seconds)
    pub timestamp: u64,
    /// Duración del período de votación en segundos (default: 72h)
    pub voting_duration_secs: u64,
    /// Firma Ed25519 del autor sobre el hash del contenido - skipped during serialization
    #[serde(skip)]
    pub signature: ed25519_dalek::Signature,
    /// Firma como hex string (for serialization)
    pub signature_hex: String,
    /// Estado actual
    pub state: ProposalState,
}

/// Internal struct for deserialization (all fields serializable)
#[derive(Deserialize)]
struct ProposalWire {
    id: Uuid,
    proposal_type: ProposalType,
    author_hex: String,
    title: String,
    payload: String,
    timestamp: u64,
    voting_duration_secs: u64,
    signature_hex: String,
    state: ProposalState,
}

impl<'de> Deserialize<'de> for Proposal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = ProposalWire::deserialize(deserializer)?;
        let author_bytes = hex::decode(&wire.author_hex)
            .map_err(|e| serde::de::Error::custom(format!("Invalid author_hex: {}", e)))?;
        let author_arr: [u8; 32] = author_bytes.try_into().map_err(|_| {
            serde::de::Error::custom("Invalid author_hex length: expected 32 bytes")
        })?;
        let author = VerifyingKey::from_bytes(&author_arr)
            .map_err(|e| serde::de::Error::custom(format!("Invalid VerifyingKey: {}", e)))?;
        let sig_bytes = hex::decode(&wire.signature_hex)
            .map_err(|e| serde::de::Error::custom(format!("Invalid signature_hex: {}", e)))?;
        let signature = ed25519_dalek::Signature::from_slice(&sig_bytes)
            .map_err(|e| serde::de::Error::custom(format!("Invalid Signature: {}", e)))?;
        Ok(Proposal {
            id: wire.id,
            proposal_type: wire.proposal_type,
            author,
            author_hex: wire.author_hex,
            title: wire.title,
            payload: wire.payload,
            timestamp: wire.timestamp,
            voting_duration_secs: wire.voting_duration_secs,
            signature,
            signature_hex: wire.signature_hex,
            state: wire.state,
        })
    }
}

impl Proposal {
    /// Crear nueva propuesta firmada
    pub fn create(
        id: Uuid,
        proposal_type: ProposalType,
        title: String,
        payload: String,
        signing_key: &SigningKey,
        voting_duration_secs: u64,
    ) -> Self {
        let verifying_key = signing_key.verifying_key();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Hash del contenido para firmar
        let content_hash = Self::compute_hash(&id, &title, &payload, timestamp);
        let signature = signing_key.sign(&content_hash);

        info!(
            proposal_id = %id,
            author = %hex::encode(verifying_key.as_bytes()),
            proposal_type = %proposal_type,
            "New proposal created"
        );

        Proposal {
            id,
            proposal_type,
            author: verifying_key,
            // FIX: trait bound - store hex representation for serialization
            author_hex: hex::encode(verifying_key.as_bytes()),
            title,
            payload,
            timestamp,
            voting_duration_secs,
            signature,
            // FIX: trait bound - store hex representation for serialization
            signature_hex: hex::encode(signature.to_bytes()),
            state: ProposalState::Proposed,
        }
    }

    /// Verificar firma de la propuesta
    pub fn verify_signature(&self) -> Result<(), ProposalError> {
        let content_hash = Self::compute_hash(&self.id, &self.title, &self.payload, self.timestamp);

        self.author
            .verify_strict(&content_hash, &self.signature)
            .map_err(|e| ProposalError::InvalidSignature(e.to_string()))?;

        Ok(())
    }

    /// Verificar que la propuesta no ha expirado
    pub fn is_voting_active(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let expiry = self.timestamp + self.voting_duration_secs;
        self.state == ProposalState::Voting && now < expiry
    }

    /// Verificar si la propuesta ha expirado
    pub fn has_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let expiry = self.timestamp + self.voting_duration_secs;
        if self.state == ProposalState::Voting && now >= expiry {
            warn!(
                proposal_id = %self.id,
                created = self.timestamp,
                expiry,
                "Proposal voting period expired"
            );
            return true;
        }
        false
    }

    /// Obtener tiempo restante de votación en segundos
    pub fn voting_time_remaining(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let expiry = self.timestamp + self.voting_duration_secs;
        expiry.saturating_sub(now)
    }

    /// Hash determinista del contenido de la propuesta
    fn compute_hash(id: &Uuid, title: &str, payload: &str, timestamp: u64) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(id.as_bytes());
        hasher.update(title.as_bytes());
        hasher.update(payload.as_bytes());
        hasher.update(timestamp.to_le_bytes());
        hasher.finalize().to_vec()
    }

    /// Generar par de claves para gobernanza
    pub fn generate_keypair() -> Result<(SigningKey, VerifyingKey), ProposalError> {
        let mut csprng = ark_std::rand::thread_rng();
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        Ok((signing_key, verifying_key))
    }

    /// Serializar propuesta a JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserializar propuesta desde JSON
    pub fn from_json(json: &str) -> Result<Self, ProposalError> {
        serde_json::from_str(json).map_err(|e| ProposalError::InvalidFormat(e.to_string()))
    }
}

/// Gestor de propuestas
pub struct ProposalManager {
    proposals: std::collections::HashMap<Uuid, Proposal>,
    default_voting_duration_secs: u64,
}

impl ProposalManager {
    pub fn new() -> Self {
        Self {
            proposals: std::collections::HashMap::new(),
            default_voting_duration_secs: 72 * 60 * 60, // 72 horas
        }
    }

    /// Registrar nueva propuesta
    pub fn submit(&mut self, proposal: Proposal) -> Result<Uuid, ProposalError> {
        // Verificar firma
        proposal.verify_signature()?;

        // FIX: borrow/move - Extract id before moving proposal
        let proposal_id = proposal.id;

        info!(
            proposal_id = %proposal_id,
            "Proposal submitted for validation"
        );

        self.proposals.insert(proposal_id, proposal);
        Ok(proposal_id)
    }

    /// Obtener propuesta por ID
    pub fn get(&self, id: &Uuid) -> Option<&Proposal> {
        self.proposals.get(id)
    }

    /// Actualizar estado de propuesta
    pub fn update_state(
        &mut self,
        id: &Uuid,
        new_state: ProposalState,
    ) -> Result<(), ProposalError> {
        let proposal = self
            .proposals
            .get_mut(id)
            .ok_or_else(|| ProposalError::InvalidFormat("Proposal not found".to_string()))?;

        let old_state = proposal.state.clone();
        proposal.state = new_state.clone();

        info!(
            proposal_id = %id,
            old_state = %old_state,
            new_state = %new_state,
            "Proposal state updated"
        );

        Ok(())
    }

    /// Obtener todas las propuestas en votación activa
    pub fn get_active_voting(&self) -> Vec<&Proposal> {
        self.proposals
            .values()
            .filter(|p| p.state == ProposalState::Voting && !p.has_expired())
            .collect()
    }

    /// Obtener propuestas aprobadas pendientes de ejecución
    pub fn get_approved_pending(&self) -> Vec<&Proposal> {
        self.proposals
            .values()
            .filter(|p| p.state == ProposalState::Approved)
            .collect()
    }

    /// Listar todas las propuestas
    pub fn list_all(&self) -> Vec<&Proposal> {
        self.proposals.values().collect()
    }

    /// Contar propuestas por estado
    pub fn count_by_state(&self) -> std::collections::HashMap<ProposalState, usize> {
        let mut counts = std::collections::HashMap::new();
        for proposal in self.proposals.values() {
            *counts.entry(proposal.state.clone()).or_insert(0) += 1;
        }
        counts
    }
}

impl Default for ProposalManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proposal_creation_and_verification() {
        let (signing_key, _verifying_key) = Proposal::generate_keypair().unwrap();
        let id = Uuid::new_v4();

        let proposal = Proposal::create(
            id,
            ProposalType::NetworkParam,
            "Test Proposal".to_string(),
            "Test payload content".to_string(),
            &signing_key,
            72 * 3600,
        );

        assert_eq!(proposal.state, ProposalState::Proposed);
        assert!(proposal.verify_signature().is_ok());
    }

    #[test]
    fn test_proposal_manager_submit() {
        let (signing_key, _) = Proposal::generate_keypair().unwrap();
        let id = Uuid::new_v4();

        let proposal = Proposal::create(
            id,
            ProposalType::Custom,
            "Manager Test".to_string(),
            "Payload".to_string(),
            &signing_key,
            72 * 3600,
        );

        let mut manager = ProposalManager::new();
        let result = manager.submit(proposal);
        assert!(result.is_ok());
        assert_eq!(manager.list_all().len(), 1);
    }

    #[test]
    fn test_proposal_json_serialization() {
        let (signing_key, _) = Proposal::generate_keypair().unwrap();
        let proposal = Proposal::create(
            Uuid::new_v4(),
            ProposalType::Governance,
            "JSON Test".to_string(),
            "Payload".to_string(),
            &signing_key,
            72 * 3600,
        );

        let json = proposal.to_json().unwrap();
        let deserialized = Proposal::from_json(&json).unwrap();
        assert_eq!(deserialized.id, proposal.id);
        assert!(deserialized.verify_signature().is_ok());
    }
}
