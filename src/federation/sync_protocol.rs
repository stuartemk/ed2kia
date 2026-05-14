//! Sync Protocol - Protocolo P2P para sincronización de deltas de pesos federados
//!
//! Define los mensajes y flujo de sincronización para actualizaciones
//! de pesos SAE entre nodos en la red ed2kIA.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

use super::avg_aggregator::{WeightUpdate, FedAvgAggregator, AggregationResult};

/// Tipo de mensaje de sincronización federada
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncMessageType {
    /// Solicitud de round de entrenamiento
    RoundRequest,
    /// Respuesta a round de entrenamiento
    RoundResponse,
    /// Actualización de pesos (delta)
    WeightUpdate,
    /// Model global agregado
    GlobalModel,
    /// Acknowledgment
    Ack,
    /// Error
    Error,
}

impl std::fmt::Display for SyncMessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncMessageType::RoundRequest => write!(f, "round_request"),
            SyncMessageType::RoundResponse => write!(f, "round_response"),
            SyncMessageType::WeightUpdate => write!(f, "weight_update"),
            SyncMessageType::GlobalModel => write!(f, "global_model"),
            SyncMessageType::Ack => write!(f, "ack"),
            SyncMessageType::Error => write!(f, "error"),
        }
    }
}

/// Mensaje de sincronización federada
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMessage {
    /// Tipo de mensaje
    pub msg_type: SyncMessageType,
    /// ID del nodo emisor
    pub sender_id: String,
    /// Round de federación
    pub round: u64,
    /// Payload del mensaje
    pub payload: SyncPayload,
    /// Timestamp Unix ms
    pub timestamp: u64,
}

impl SyncMessage {
    pub fn new(msg_type: SyncMessageType, sender_id: String, round: u64, payload: SyncPayload) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            msg_type,
            sender_id,
            round,
            payload,
            timestamp,
        }
    }

    pub fn round_request(sender_id: String, round: u64, min_participants: usize) -> Self {
        Self::new(
            SyncMessageType::RoundRequest,
            sender_id,
            round,
            SyncPayload::RoundRequest { min_participants },
        )
    }

    pub fn weight_update(sender_id: String, round: u64, update: WeightUpdate) -> Self {
        Self::new(
            SyncMessageType::WeightUpdate,
            sender_id,
            round,
            SyncPayload::WeightUpdate(update),
        )
    }

    pub fn global_model(sender_id: String, round: u64, result: AggregationResult) -> Self {
        Self::new(
            SyncMessageType::GlobalModel,
            sender_id,
            round,
            SyncPayload::GlobalModel(result),
        )
    }

    pub fn error(sender_id: String, round: u64, error_msg: String) -> Self {
        Self::new(
            SyncMessageType::Error,
            sender_id,
            round,
            SyncPayload::Error(error_msg),
        )
    }
}

/// Payload del mensaje de sincronización
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncPayload {
    /// Solicitud de round
    RoundRequest { min_participants: usize },
    /// Respuesta a round (nodo acepta/participa)
    RoundResponse { accepts: bool, reason: Option<String> },
    /// Actualización de pesos
    WeightUpdate(WeightUpdate),
    /// Modelo global agregado
    GlobalModel(AggregationResult),
    /// Acknowledgment
    Ack { msg_id: String },
    /// Error
    Error(String),
}

/// Estado del round de federación
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoundState {
    /// Esperando participantes
    Waiting,
    /// Recibiendo actualizaciones
    Collecting,
    /// Agregando
    Aggregating,
    /// Distribuyendo modelo global
    Distributing,
    /// Completado
    Completed,
    /// Fallido
    Failed(String),
}

/// Round de federación activo
#[derive(Debug, Clone)]
pub struct FederationRound {
    pub round_id: u64,
    pub state: RoundState,
    pub layer_id: u32,
    pub min_participants: usize,
    pub participants: Vec<String>,
    pub updates_received: Vec<WeightUpdate>,
    pub result: Option<AggregationResult>,
    pub started_at: u64,
}

impl FederationRound {
    pub fn new(round_id: u64, layer_id: u32, min_participants: usize) -> Self {
        Self {
            round_id,
            state: RoundState::Waiting,
            layer_id,
            min_participants,
            participants: Vec::new(),
            updates_received: Vec::new(),
            result: None,
            started_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self.state, RoundState::Completed | RoundState::Failed(_))
    }
}

/// Protocolo de sincronización federada
pub struct SyncProtocol {
    aggregator: FedAvgAggregator,
    current_round: u64,
    active_rounds: HashMap<u64, FederationRound>,
}

impl SyncProtocol {
    pub fn new(aggregator: FedAvgAggregator) -> Self {
        Self {
            aggregator,
            current_round: 0,
            active_rounds: HashMap::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(FedAvgAggregator::with_defaults())
    }

    /// Iniciar nuevo round de federación
    pub fn start_round(&mut self, layer_id: u32, min_participants: usize) -> u64 {
        self.current_round += 1;
        let round = FederationRound::new(self.current_round, layer_id, min_participants);

        info!(
            "Round {} iniciado: layer={}, min_participants={}",
            self.current_round, layer_id, min_participants
        );

        self.active_rounds.insert(self.current_round, round);
        self.current_round
    }

    /// Procesar mensaje de sincronización
    pub fn process_message(&mut self, message: SyncMessage) -> Result<Option<SyncMessage>> {
        debug!(
            "Procesando mensaje {} de {} (round={})",
            message.msg_type, message.sender_id, message.round
        );

        match message.msg_type {
            SyncMessageType::RoundRequest => {
                self.handle_round_request(message)
            }
            SyncMessageType::WeightUpdate => {
                self.handle_weight_update(message)
            }
            SyncMessageType::GlobalModel => {
                self.handle_global_model(message)
            }
            SyncMessageType::Ack => {
                Ok(None)
            }
            SyncMessageType::Error => {
                warn!("Error recibido: {:#?}", message.payload);
                Ok(None)
            }
            SyncMessageType::RoundResponse => {
                self.handle_round_response(message)
            }
        }
    }

    /// Manejar solicitud de round
    fn handle_round_request(&mut self, message: SyncMessage) -> Result<Option<SyncMessage>> {
        if let SyncPayload::RoundRequest { min_participants } = &message.payload {
            let _round = self.start_round(0, *min_participants);
            let response = SyncMessage::new(
                SyncMessageType::RoundResponse,
                message.sender_id.clone(),
                message.round,
                SyncPayload::RoundResponse {
                    accepts: true,
                    reason: None,
                },
            );
            Ok(Some(response))
        } else {
            Err(anyhow::anyhow!("Invalid RoundRequest payload"))
        }
    }

    /// Manejar respuesta de round
    fn handle_round_response(&mut self, message: SyncMessage) -> Result<Option<SyncMessage>> {
        if let Some(round) = self.active_rounds.get_mut(&message.round) {
            if !round.participants.contains(&message.sender_id) {
                round.participants.push(message.sender_id.clone());
                info!("Participante {} unido al round {}", message.sender_id, message.round);

                if round.participants.len() >= round.min_participants {
                    round.state = RoundState::Collecting;
                    info!("Round {} tiene suficientes participantes", message.round);
                }
            }
        }
        Ok(None)
    }

    /// Manejar actualización de pesos
    fn handle_weight_update(&mut self, message: SyncMessage) -> Result<Option<SyncMessage>> {
        if let SyncPayload::WeightUpdate(update) = &message.payload {
            // Almacenar en aggregator
            self.aggregator.add_update(update.clone())?;

            // Almacenar en round activo
            if let Some(round) = self.active_rounds.get_mut(&message.round) {
                round.updates_received.push(update.clone());

                // Verificar si tenemos suficientes actualizaciones
                if round.updates_received.len() >= round.min_participants {
                    // Ejecutar agregación
                    match self.aggregator.aggregate(round.layer_id) {
                        Ok(result) => {
                            round.result = Some(result.clone());
                            round.state = RoundState::Completed;

                            let global_msg = SyncMessage::global_model(
                                message.sender_id.clone(),
                                message.round,
                                result,
                            );
                            return Ok(Some(global_msg));
                        }
                        Err(e) => {
                            round.state = RoundState::Failed(e.to_string());
                            warn!("Agregación fallida en round {}: {}", message.round, e);
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    /// Manejar modelo global
    fn handle_global_model(&mut self, message: SyncMessage) -> Result<Option<SyncMessage>> {
        if let SyncPayload::GlobalModel(result) = &message.payload {
            info!(
                "Modelo global recibido para round {}: {} nodos, {} excluidos",
                message.round,
                result.included_nodes.len(),
                result.excluded_nodes.len()
            );

            // Ack
            let ack = SyncMessage::new(
                SyncMessageType::Ack,
                message.sender_id.clone(),
                message.round,
                SyncPayload::Ack {
                    msg_id: format!("global_{}", message.round),
                },
            );
            Ok(Some(ack))
        } else {
            Err(anyhow::anyhow!("Invalid GlobalModel payload"))
        }
    }

    /// Obtener round activo
    pub fn get_active_round(&self, round_id: u64) -> Option<&FederationRound> {
        self.active_rounds.get(&round_id)
    }

    /// Limpiar rounds completados
    pub fn cleanup_completed(&mut self) {
        let completed: Vec<u64> = self.active_rounds
            .iter()
            .filter(|(_, r)| r.is_complete())
            .map(|(&id, _)| id)
            .collect();

        for id in completed {
            self.active_rounds.remove(&id);
            self.aggregator.clear_layer(id as u32);
            debug!("Round {} limpiado", id);
        }
    }

    /// Estadísticas del protocolo
    pub fn stats(&self) -> ProtocolStats {
        let active: Vec<&FederationRound> = self.active_rounds.values().collect();
        ProtocolStats {
            current_round: self.current_round,
            active_rounds: active.len(),
            total_participants: active.iter().map(|r| r.participants.len()).sum(),
            pending_updates: self.aggregator.pending_layers().len(),
        }
    }
}

/// Estadísticas del protocolo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolStats {
    pub current_round: u64,
    pub active_rounds: usize,
    pub total_participants: usize,
    pub pending_updates: usize,
}

impl Default for SyncProtocol {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::federation::avg_aggregator::WeightUpdate;

    fn make_update(node_id: &str, _round: u64) -> WeightUpdate {
        let deltas: Vec<f32> = (0..100).map(|i| i as f32 / 50.0 - 1.0).collect();
        WeightUpdate::new(node_id.to_string(), 0, deltas, 100, 0.5)
    }

    #[test]
    fn test_sync_message_creation() {
        let msg = SyncMessage::round_request("coordinator".to_string(), 1, 3);
        assert_eq!(msg.msg_type, SyncMessageType::RoundRequest);
        assert_eq!(msg.round, 1);
    }

    #[test]
    fn test_protocol_start_round() {
        let mut protocol = SyncProtocol::with_defaults();
        let round_id = protocol.start_round(0, 3);
        assert_eq!(round_id, 1);
        assert!(protocol.get_active_round(1).is_some());
    }

    #[test]
    fn test_protocol_full_flow() {
        let mut protocol = SyncProtocol::with_defaults();

        // Start round
        let round_id = protocol.start_round(0, 3);

        // Send weight updates
        for i in 0..3 {
            let update = make_update(&format!("node{}", i), round_id);
            let msg = SyncMessage::weight_update(
                format!("node{}", i),
                round_id,
                update,
            );
            protocol.process_message(msg).unwrap();
        }

        let stats = protocol.stats();
        assert_eq!(stats.current_round, round_id);
    }

    #[test]
    fn test_protocol_stats() {
        let protocol = SyncProtocol::with_defaults();
        let stats = protocol.stats();
        assert_eq!(stats.current_round, 0);
        assert_eq!(stats.active_rounds, 0);
    }

    #[test]
    fn test_round_states() {
        let round = FederationRound::new(1, 0, 3);
        assert_eq!(round.state, RoundState::Waiting);
        assert!(!round.is_complete());

        let mut failed_round = round.clone();
        failed_round.state = RoundState::Failed("error".to_string());
        assert!(failed_round.is_complete());
    }
}
