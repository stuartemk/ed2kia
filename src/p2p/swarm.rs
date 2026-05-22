//! Swarm P2P - Inicialización y gestión de la red libp2p
//!
//! Maneja la creación del nodo, descubrimiento de peers (KAD + mDNS),
//! comunicación request-response y pubsub para señales de steering.

use anyhow::{Context, Result};
use futures::StreamExt;
use libp2p::{
    gossipsub::{self},
    identity::Keypair,
    kad::{self},
    mdns,
    request_response::{self, cbor},
    swarm::{NetworkBehaviour, SwarmEvent},
    Multiaddr, PeerId, Swarm, SwarmBuilder,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

use super::protocol::{Ed2kMessage, Ed2kMessageCodec, TensorRequest, TensorResponse};

// ============================================================================
// Fase 2: GossipSub Topics
// ============================================================================

/// Topic para broadcast de FeatureBatch
pub const FEATURE_BATCH_TOPIC: &str = "ed2kia.feature-batch";

/// Topic para steering signals
pub const STEERING_SIGNAL_TOPIC: &str = "ed2kia.steering";

/// Topic para votos de consenso
pub const CONSENSUS_VOTE_TOPIC: &str = "ed2kia.consensus-vote";

/// Tiempo de lease para sharding de capas SAE (5 minutos)
pub const LAYER_LEASE_DURATION: Duration = Duration::from_secs(300);

/// Intervalo de renovación de lease (30 segundos antes de expirar)
pub const LEASE_RENEWAL_INTERVAL: Duration = Duration::from_secs(30);

/// Comportamiento de la red P2P
// MIGRATION: libp2p 0.53 - use cbor::codec::Codec<Req, Resp> as Codec type
#[derive(NetworkBehaviour)]
pub struct Ed2kBehaviour {
    /// Descubrimiento via mDNS (local network)
    mdns: mdns::tokio::Behaviour,
    /// Kademlia para descubrimiento global y DHT
    kad: kad::Behaviour<kad::store::MemoryStore>,
    // MIGRATION: libp2p 0.53 - use cbor::Behaviour directly
    request_response: request_response::cbor::Behaviour<Ed2kMessage, Ed2kMessage>,
    /// Fase 2: GossipSub para broadcast de FeatureBatch, SteeringSignals y ConsensusVotes
    pubsub: gossipsub::Behaviour,
}

/// Estado del nodo
#[derive(Debug, Clone)]
pub struct NodeStatus {
    pub node_id: String,
    pub peer_count: usize,
    pub sae_layers: usize,
    pub active_leases: usize,
}

/// Recurso computacional del nodo (para sharding dinámico)
#[derive(Debug, Clone)]
pub struct NodeResources {
    pub cpu_cores: usize,
    pub available_ram_gb: f64,
    pub bandwidth_mbps: f64,
    pub avg_latency_ms: f64,
    pub has_gpu: bool,
    pub gpu_model: Option<String>,
    pub vram_gb: Option<f64>,
}

/// Canal interno para eventos del swarm
pub struct Ed2kSwarm {
    swarm: Swarm<Ed2kBehaviour>,
    node_id: PeerId,
    connected_peers: HashMap<PeerId, PeerInfo>,
    lease_expirations: HashMap<PeerId, Instant>,
    /// Fase 2: Tabla de reputación de peers
    peer_reputation: HashMap<PeerId, f64>,
    /// Fase 2: Topics de gossipsub suscritos
    subscribed_topics: Vec<gossipsub::IdentTopic>,
}

/// Información de un peer conectado
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub peer_id: PeerId,
    pub addresses: Vec<Multiaddr>,
    pub connected_at: Instant,
    pub resources: Option<NodeResources>,
    pub assigned_layers: Vec<u32>,
}

impl Ed2kSwarm {
    /// Crear nuevo swarm P2P
    pub async fn new(node_id: Option<String>, port: u16) -> Result<Self> {
        // Generar o usar keypair existente
        let keypair = match node_id {
            Some(id_str) => {
                info!("Usando node_id proporcionado: {}", id_str);
                Keypair::generate_ed25519()
            }
            None => {
                info!("Generando nuevo keypair para el nodo");
                Keypair::generate_ed25519()
            }
        };

        let local_peer_id = PeerId::from(keypair.public());
        info!("Node ID: {}", local_peer_id);

        // Fase 2: Configurar GossipSub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .validation_mode(gossipsub::ValidationMode::Permissive)
            .mesh_n(6)
            .mesh_n_low(4)
            .mesh_n_high(12)
            .build()
            .context("Invalid gossipsub config")?;

        // MIGRATION: StrictSigned -> Signed in libp2p 0.53
        let pubsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create gossipsub behaviour: {}", e))?;

        // MIGRATION: Swarm::with_single_interface removed in libp2p 0.53, use Swarm::new()
        let behaviour = Ed2kBehaviour {
            // MIGRATION: mdns::new() now requires PeerId as second argument
            mdns: mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)
                .context("Failed to initialise mDNS behaviour")?,
            kad: kad::Behaviour::new(local_peer_id, kad::store::MemoryStore::new(local_peer_id)),
            // MIGRATION: libp2p 0.53 - use cbor::Behaviour with protocol paths
            request_response: {
                let protocol = Ed2kMessageCodec::protocol_name();
                let config = request_response::Config::default();
                cbor::Behaviour::new(
                    [(protocol, request_response::ProtocolSupport::Full)],
                    config,
                )
            },
            pubsub,
        };

        // MIGRATION: libp2p 0.53 - Swarm::new() removed, use SwarmBuilder
        let mut swarm = SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_tcp(
                Default::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .unwrap()
            // MIGRATION: libp2p 0.53 - with_behaviour requires a closure
            .with_behaviour(|_| behaviour)
            .unwrap()
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        // Escuchar en puerto especificado (0 = auto)
        let listen_addr = format!("/ip4/0.0.0.0/tcp/{}", port);
        let listen_addr: Multiaddr = listen_addr.parse().context("Invalid listen address")?;
        swarm.listen_on(listen_addr.clone())?;
        info!("Escuchando en: {}", listen_addr);

        Ok(Self {
            swarm,
            node_id: local_peer_id,
            connected_peers: HashMap::new(),
            lease_expirations: HashMap::new(),
            peer_reputation: HashMap::new(),
            subscribed_topics: Vec::new(),
        })
    }

    /// Fase 2: Suscribirse a topics de gossipsub
    pub fn subscribe_all_topics(&mut self) -> Result<()> {
        let topics = [
            FEATURE_BATCH_TOPIC,
            STEERING_SIGNAL_TOPIC,
            CONSENSUS_VOTE_TOPIC,
        ];

        for topic_str in &topics {
            let topic = gossipsub::IdentTopic::new(topic_str.to_string());
            match self.swarm.behaviour_mut().pubsub.subscribe(&topic) {
                Ok(propagation_delta) => {
                    info!(
                        "Suscrito a topic: {} (delta: {:?})",
                        topic_str, propagation_delta
                    );
                    self.subscribed_topics.push(topic);
                }
                Err(e) => {
                    warn!("Error suscribiéndose a {}: {:?}", topic_str, e);
                }
            }
        }

        Ok(())
    }

    /// Fase 2: Publicar mensaje via gossipsub
    pub fn publish_gossipsub(&mut self, topic: &str, data: Vec<u8>) -> Result<()> {
        let topic_str = topic.to_string();
        let gossip_topic = gossipsub::IdentTopic::new(&topic_str);
        match self
            .swarm
            .behaviour_mut()
            .pubsub
            .publish(gossip_topic, data)
        {
            Ok(_) => {
                debug!("Mensaje publicado a topic: {}", topic_str);
                Ok(())
            }
            Err(e) => {
                warn!("Error publicando a {}: {:?}", topic, e);
                Err(anyhow::anyhow!("Gossipsub publish failed: {:?}", e))
            }
        }
    }

    /// Fase 2: Actualizar reputación de peer
    pub fn update_peer_reputation(&mut self, peer_id: PeerId, score: f64) {
        self.peer_reputation
            .entry(peer_id)
            .and_modify(|s| *s = (*s + score) / 2.0) // Moving average
            .or_insert(score);

        // MIGRATION: report_peer(PeerReputationMessage) removed in libp2p 0.53, use validate_message method instead
        // Peer reputation is now tracked internally via gossipsub's scoring system

        // Si reputación muy baja, considerar revocar lease
        if let Some(rep) = self.peer_reputation.get(&peer_id) {
            if *rep < -0.5 {
                warn!(
                    "Peer {} con reputación baja: {:.3} - revocando lease",
                    peer_id, rep
                );
                self.lease_expirations.remove(&peer_id);
            }
        }
    }

    /// Procesar eventos del swarm (event loop)
    pub async fn event_loop(
        &mut self,
        max_peers: usize,
        // TODO: Phase 2 - Integrar LayerRouter y SAELoader
        // router: &mut crate::sae::router::LayerRouter,
        // loader: Option<&crate::sae::loader::SAELoader>,
    ) -> Result<()> {
        info!("Event loop iniciado (max_peers={})", max_peers);

        // MIGRATION: next_event() -> select_next_some() in libp2p 0.53
        loop {
            match self.swarm.select_next_some().await {
                // MIGRATION: NewListen -> ListenerNewAddress in libp2p 0.53
                // MIGRATION: libp2p 0.53 - ListenerNewAddress → NewListenAddress
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("Escuchando en: {}", address);
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    info!("Conexión establecida con: {}", peer_id);
                    self.connected_peers
                        .entry(peer_id)
                        .or_insert_with(|| PeerInfo {
                            peer_id,
                            addresses: Vec::new(),
                            connected_at: Instant::now(),
                            resources: None,
                            assigned_layers: Vec::new(),
                        });

                    // Limitar peers
                    if self.connected_peers.len() > max_peers {
                        warn!("Límite de peers alcanzado ({})", max_peers);
                        // TODO: Phase 2 - Implementar eviction strategy
                    }
                }
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    info!("Conexión cerrada con: {}", peer_id);
                    self.connected_peers.remove(&peer_id);
                    self.lease_expirations.remove(&peer_id);
                }
                // MIGRATION: mDNS event structure changed in libp2p 0.53
                // MIGRATION: libp2p 0.53 - Discovered contains Vec<(PeerId, Multiaddr)>
                SwarmEvent::Behaviour(Ed2kBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, multiaddr) in list {
                        debug!("mDNS discovered peer {} at: {}", peer_id, multiaddr);
                        if self.connected_peers.len() < max_peers {
                            self.swarm.dial(multiaddr).ok();
                        }
                    }
                }
                // MIGRATION: kad::QueryResult::GetClosestPeers -> GetClosestPeers result has .peers() method
                SwarmEvent::Behaviour(Ed2kBehaviourEvent::Kad(
                    kad::Event::OutboundQueryProgressed {
                        result: kad::QueryResult::GetClosestPeers(Ok(result)),
                        ..
                    },
                )) => {
                    // MIGRATION: libp2p 0.53 - result.peers() → result.peers (field)
                    for peer in result.peers {
                        debug!("KAD closest peer: {}", peer);
                    }
                }
                // MIGRATION: request_response::Event::Message -> InboundResponse/ResponseReceived in libp2p 0.53
                SwarmEvent::Behaviour(Ed2kBehaviourEvent::RequestResponse(
                    request_response::Event::Message { peer, message },
                )) => {
                    debug!("Mensaje recibido de: {}", peer);
                    // MIGRATION: libp2p 0.53 - message is Message<Req, Resp> enum
                    if let request_response::Message::Request { request, .. } = message {
                        self.handle_message(peer, request).await?;
                    }
                }
                SwarmEvent::Behaviour(Ed2kBehaviourEvent::RequestResponse(
                    request_response::Event::ResponseSent { peer, .. },
                )) => {
                    debug!("Respuesta enviada a: {}", peer);
                }
                SwarmEvent::Behaviour(Ed2kBehaviourEvent::RequestResponse(
                    request_response::Event::OutboundFailure { error, .. },
                )) => {
                    error!("Error en solicitud saliente: {:?}", error);
                }
                // MIGRATION: IncomingRequest removed in libp2p 0.53, handled via Message event
                // ─── Fase 2: GossipSub Events ───
                SwarmEvent::Behaviour(Ed2kBehaviourEvent::Pubsub(gossipsub::Event::Message {
                    propagation_source,
                    message,
                    ..
                })) => {
                    debug!(
                        "GossipSub message de {}: topic={}, size={} bytes",
                        propagation_source,
                        message.topic,
                        message.data.len()
                    );

                    // Deserializar mensaje
                    if let Ok(ed2k_msg) = bincode::deserialize::<Ed2kMessage>(&message.data) {
                        self.handle_message(propagation_source, ed2k_msg).await?;
                    } else {
                        warn!("Error deserializando GossipSub message");
                        // Penalizar peer por enviar datos inválidos
                        self.update_peer_reputation(propagation_source, -0.1);
                    }
                }
                SwarmEvent::Behaviour(Ed2kBehaviourEvent::Pubsub(
                    gossipsub::Event::Subscribed { peer_id, topic },
                )) => {
                    debug!("Peer {} suscrito a {}", peer_id, topic);
                }
                SwarmEvent::Behaviour(Ed2kBehaviourEvent::Pubsub(
                    gossipsub::Event::Unsubscribed { peer_id, topic },
                )) => {
                    debug!("Peer {} desuscrito de {}", peer_id, topic);
                }
                _ => {}
            }

            // TODO: Phase 2 - Renovar leases expirados
            // self.renew_expired_leases().await?;
        }
    }

    /// Manejar mensaje recibido de un peer
    async fn handle_message(&mut self, peer: PeerId, message: Ed2kMessage) -> Result<()> {
        match message {
            Ed2kMessage::TensorRequest(req) => {
                info!(
                    "TensorRequest de {}: layer={}, shape={:?}",
                    peer, req.layer_id, req.tensor_shape
                );
                // TODO: Phase 2 - Procesar SAE inference y enviar respuesta
                let response = TensorResponse {
                    request_id: req.request_id,
                    layer_id: req.layer_id,
                    sparse_features: vec![], // Placeholder
                    confidence_score: 0.0,
                    error: None,
                };
                let _ = self
                    .swarm
                    .behaviour_mut()
                    .request_response
                    .send_request(&peer, Ed2kMessage::TensorResponse(response));
            }
            Ed2kMessage::TensorResponse(resp) => {
                info!(
                    "TensorResponse para request {}: confidence={}",
                    resp.request_id, resp.confidence_score
                );
                // TODO: Phase 2 - Agregar sparse features al pipeline
            }
            Ed2kMessage::LeaseRequest(lease_req) => {
                info!("LeaseRequest de {}: layers={:?}", peer, lease_req.layers);
                // TODO: Phase 2 - Aprovar/denegar lease basado en recursos
            }
            Ed2kMessage::LeaseResponse(lease_resp) => {
                info!(
                    "LeaseResponse: granted={}, expires={:?}",
                    lease_resp.granted, lease_resp.expires_at
                );
                if lease_resp.granted {
                    self.lease_expirations.insert(
                        // TODO: Phase 2 - Track peer correctly
                        *self.connected_peers.keys().next().unwrap_or(&self.node_id),
                        Instant::now() + LAYER_LEASE_DURATION,
                    );
                }
            }
            Ed2kMessage::SteeringSignal(signal) => {
                debug!(
                    "SteeringSignal: type={}, payload={}",
                    signal.signal_type, signal.payload
                );
                // TODO: Phase 2 - Procesar steering signals (atención, temperatura, etc.)
            }
            Ed2kMessage::ResourceAdvertisement(resources) => {
                info!("ResourceAdvertisement de: {:?}", resources);
                if let Some(peer_info) = self.connected_peers.get_mut(&peer) {
                    // MIGRATION: protocol::NodeResources vs swarm::NodeResources - use protocol version
                    peer_info.resources = Some(crate::p2p::swarm::NodeResources {
                        cpu_cores: resources.cpu_cores,
                        available_ram_gb: resources.available_ram_gb,
                        bandwidth_mbps: resources.bandwidth_mbps,
                        avg_latency_ms: resources.avg_latency_ms,
                        has_gpu: resources.has_gpu,
                        gpu_model: resources.gpu_model.clone(),
                        vram_gb: resources.vram_gb,
                    });
                }
            }
            // ─── Fase 2: Nuevos mensajes ───
            Ed2kMessage::FeatureBatch(batch) => {
                info!(
                    "FeatureBatch de {}: batch={}, layer={}, features={}",
                    peer,
                    batch.batch_id,
                    batch.layer_id,
                    batch.features.len()
                );
                // TODO: Phase 2 - Enviar a ConsensusValidator
                // validator.receive_vote(ConsensusVote { ... })?;
            }
            Ed2kMessage::ConsensusVote(vote) => {
                info!(
                    "ConsensusVote de {}: batch={}, confidence={}",
                    peer, vote.batch_id, vote.confidence
                );
                // TODO: Phase 2 - Enviar a ConsensusValidator
                // validator.receive_vote(vote)?;
            }
            Ed2kMessage::AnalysisResult(result) => {
                debug!(
                    "AnalysisResult de {}: anomaly={:.3}, confidence={:.3}",
                    result.analyzer_peer_id, result.anomaly_score, result.confidence
                );
                // TODO: Phase 2 - Enviar a ConsciousnessBridge
            }
        }
        Ok(())
    }

    /// Enviar TensorRequest a un peer específico
    pub fn send_tensor_request(&mut self, peer: &PeerId, req: TensorRequest) -> Result<()> {
        self.swarm
            .behaviour_mut()
            .request_response
            .send_request(peer, Ed2kMessage::TensorRequest(req));
        Ok(())
    }

    /// Obtener estado actual del nodo
    pub async fn get_status(&self) -> Result<NodeStatus> {
        Ok(NodeStatus {
            node_id: self.node_id.to_string(),
            peer_count: self.connected_peers.len(),
            sae_layers: 0, // TODO: Phase 2 - Contar capas asignadas
            active_leases: self.lease_expirations.len(),
        })
    }

    /// Salida ordenada de la red
    pub async fn graceful_exit(&mut self) -> Result<()> {
        info!(
            "Enviando señales de salida a {} peers...",
            self.connected_peers.len()
        );

        // TODO: Phase 2 - Notificar a peers sobre liberación de leases
        // TODO: Phase 2 - Transferir leases activos a otros peers

        // Cerrar conexiones
        for peer_id in self.connected_peers.keys() {
            self.swarm.behaviour_mut().request_response.send_request(
                peer_id,
                Ed2kMessage::SteeringSignal(crate::p2p::protocol::SteeringSignal {
                    signal_type: "node_exit".to_string(),
                    payload: format!("Node {} exiting gracefully", self.node_id),
                    priority: 100,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                }),
            );
        }

        info!("Salida ordenada completada");
        Ok(())
    }

    // TODO: Phase 2 - Conectar a bootstrap peer
    // pub async fn connect_bootstrap(&mut self, multiaddr: &str) -> Result<()> {
    //     let addr: Multiaddr = multiaddr.parse()?;
    //     self.swarm.dial(addr)?;
    //     Ok(())
    // }

    // TODO: Phase 2 - Renovar leases expirados
    // async fn renew_expired_leases(&mut self) -> Result<()> {
    //     let now = Instant::now();
    //     for (peer_id, expires_at) in &self.lease_expirations {
    //         if now.duration_since(*expires_at) > LEASE_RENEWAL_INTERVAL {
    //             // Enviar LeaseRequest de renovación
    //         }
    //     }
    //     Ok(())
    // }
}
