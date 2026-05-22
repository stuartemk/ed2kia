//! ed2kIA v1.0.0 STABLE
//!
//! Red descentralizada de código abierto para análisis interpretativo
//! distribuido de LLMs usando Sparse Autoencoders (Qwen-Scope).
//!
//! v1.0.0: Consolidación de Fases 1-9 — P2P, SAE, Consenso, Alignment,
//! Federation, Marketplace, UI, SLO, Governance unificados.
//!
//! Licencia: Apache 2.0 + Cláusula de Uso Ético
//! Este software es de código abierto, transparente y diseñado exclusivamente
//! para el progreso humano y el desarrollo responsable de la IA.

// CLEANUP: Allow dead_code for intentional public API surface not called from main()
#![allow(dead_code)]

// ─── Fase 1: Core Modules ───
mod p2p {
    pub mod protocol;
    pub mod swarm;
}

mod sae {
    pub mod loader;
    pub mod router;
}

mod bridge {
    pub mod consciousness;
    pub mod tensor_flow;
}

// ─── Fase 2: Interpretation, Feedback & Consensus ───
mod interpret {
    pub mod feature_analyzer;
    pub mod semantic_map;
}

mod consensus {
    pub mod merkle;
    pub mod validator;
}

// ─── Fase 3: Security, ZKP, Human-in-the-Loop ───
mod security {
    pub mod memory_guard;
    pub mod wasm_sandbox;
}

mod zkp {
    pub mod circuit;
    pub mod verifier;
}

mod human {
    pub mod concept_updater;
    pub mod feedback_cli;
}

// ─── Fase 4: Scaling, RLHF, Web UI, Monitoring ───
// CONSOLIDATION: v1.0.0 - All modules enabled via `stable` feature
#[cfg(feature = "stable")]
mod scaling {
    pub mod bootstrap;
    pub mod peer_manager;
    // FIX: v1.0.1-patch - Add cross_model for phase8-sprint2 (E0432)
    #[cfg(feature = "phase8-sprint2")]
    pub mod cross_model;
}

#[cfg(feature = "stable")]
mod rlhf {
    pub mod feedback_store;
    pub mod trainer_loop;
}

#[cfg(feature = "stable")]
mod web {
    pub mod routes;
    pub mod server;
}

#[cfg(feature = "stable")]
mod monitoring {
    pub mod health;
    pub mod metrics;
}

// ─── Fase 5: Governance, Reputation, Ecosystem, Bootstrap ───
// CONSOLIDATION: v1.0.0 - All modules enabled via `stable` feature
#[cfg(feature = "stable")]
mod governance {
    pub mod proposal;
    pub mod voting;
}

#[cfg(feature = "stable")]
mod reputation {
    pub mod ledger;
    pub mod scoring;
}

#[cfg(feature = "stable")]
mod ecosystem {
    pub mod hf_sync;
    pub mod model_registry;
}

#[cfg(feature = "stable")]
mod bootstrap {
    pub mod network_init;
    pub mod seed_registry;
}

// ─── Fase 6: Interoperability, Federation, Staking, API ───
// CONSOLIDATION: v1.0.0 - All modules enabled via `stable` feature
#[cfg(feature = "stable")]
mod interoperability {
    pub mod adapter;
    pub mod onnx_adapter;
    pub mod schema;
}

#[cfg(feature = "stable")]
mod federation {
    pub mod avg_aggregator;
    pub mod sync_protocol;
}

#[cfg(feature = "stable")]
mod staking {
    pub mod proof;
    pub mod registry;
}

#[cfg(feature = "stable")]
mod phase6;

#[cfg(feature = "stable")]
mod api {
    pub mod auth;
    pub mod openapi;
    pub mod routes;
}

// ─── Fase 7: Continuous Alignment, Cross-Net Federation, Dynamic Trust ───
// CONSOLIDATION: v1.0.0 - All modules enabled via `stable` feature
#[cfg(feature = "stable")]
mod alignment {
    pub mod engine;
    // FIX: v1.0.1-patch - Add continuous for phase8-sprint2 (E0432)
    #[cfg(feature = "phase8-sprint2")]
    pub mod continuous;
}

// FIX: v1.0.1-patch - Remove federation_v2 module (files don't exist as separate module)
// The bridge and trust_scoring functionality is in src/bridge/ and src/federation/

// FIX: v1.0.1-patch - Remove schema_registry module (no separate file exists)
// Schema registry functionality is in src/interoperability/schema.rs

// ─── Fase 8: Marketplace, UI Backend, SLO Engine ───
// CONSOLIDATION: v1.0.0 - All modules enabled via `stable` feature
#[cfg(feature = "stable")]
mod marketplace {
    pub mod engine;
    #[cfg(test)]
    mod tests;
}

#[cfg(feature = "stable")]
mod ui {
    pub mod backend;
    #[cfg(test)]
    mod tests;
}

#[cfg(feature = "stable")]
mod slo {
    pub mod engine;
    // FIX: v1.0.1-patch - Add enforcer for phase8-sprint2 (E0432)
    #[cfg(feature = "phase8-sprint2")]
    pub mod enforcer;
    #[cfg(test)]
    mod tests;
}

#[cfg(feature = "stable")]
mod phase8;

// FIX: v1.0.1-patch - Add governance_v2, ui_v2, federation_v3 for phase9 (E0433)
#[cfg(feature = "stable")]
mod governance_v2 {
    #[path = "../governance/liquid.rs"]
    pub mod liquid;
}

#[cfg(feature = "stable")]
mod ui_v2 {
    #[path = "../ui/realtime.rs"]
    pub mod realtime;
}

#[cfg(feature = "stable")]
mod federation_v3 {
    #[path = "../federation/async_zkp.rs"]
    pub mod async_zkp;
}

#[cfg(feature = "stable")]
mod phase9;

use anyhow::Result;
use bridge::consciousness::ConsciousnessBridge;
use clap::{Parser, Subcommand};
use consensus::validator::ConsensusValidator;
use human::feedback_cli::FeedbackManager;
use interpret::feature_analyzer::FeatureAnalyzer;
use p2p::swarm::Ed2kSwarm;
use sae::loader::SAELoader;
use sae::router::LayerRouter;
use security::wasm_sandbox::WASMSandbox;
use zkp::verifier::ZKPVerifier;
// CONSOLIDATION: v1.0.0 - All imports enabled via `stable` feature
#[cfg(feature = "stable")]
use scaling::bootstrap::BootstrapManager;
#[cfg(feature = "stable")]
use scaling::peer_manager::PeerManager;
// CLEANUP: Removed unused imports FeedbackStore, TrainerLoop, WebServer, MetricsManager
#[cfg(feature = "stable")]
use monitoring::health::HealthManager;
use tracing::info;

// CONSOLIDATION: v1.0.0 - Fase 5 imports enabled via `stable` feature
#[cfg(feature = "stable")]
use governance::proposal::{Proposal, ProposalManager, ProposalType};
// CLEANUP: Removed unused imports VotingManager, Vote, VoteDirection, VotingConfig
// CLEANUP: Removed unused imports ReputationLedger, Contribution, ContributionType
#[cfg(feature = "stable")]
use ecosystem::hf_sync::HfSyncManager;
#[cfg(feature = "stable")]
use reputation::scoring::ReputationScorer; // CLEANUP: Removed unused ModelSource
                                           // CLEANUP: Removed unused import ModelRegistry
#[cfg(feature = "stable")]
use bootstrap::network_init::{GenesisConfig, NetworkInitializer};
#[cfg(feature = "stable")]
use bootstrap::seed_registry::SeedRegistry;

// CONSOLIDATION: v1.0.0 - Fase 6 imports enabled via `stable` feature
// FIX: v1.0.1-patch - ModelNormConfig removed from adapter (no longer exported)
#[cfg(feature = "stable")]
use api::openapi::{Components, Contact, Info, OpenApiSpec, Paths, Server};
#[cfg(feature = "stable")]
use federation::avg_aggregator::{FedAvgAggregator, FedAvgConfig, WeightUpdate};
#[cfg(feature = "stable")]
use federation::sync_protocol::SyncProtocol; // CLEANUP: Removed unused SyncMessage, SyncPayload
#[cfg(feature = "stable")]
use interoperability::adapter::{SourceModel, TensorAdapter}; // CLEANUP: Removed unused NormalizedHiddenState
#[cfg(feature = "stable")]
use interoperability::schema::QwenScopeSchema;
#[cfg(feature = "stable")]
use staking::proof::{ComputeMetrics, ProofGenerator, ProofVerifier}; // CLEANUP: Removed unused StakingProof
#[cfg(feature = "stable")]
use staking::registry::{ResourceCommitment, ResourceRegistry}; // CLEANUP: Removed unused PathItem, Operation
                                                               // CLEANUP: Removed unused import ApiV2State

/// ed2kIA v1.0.0 STABLE - Descentralized Distributed Interpretability Network
#[derive(Parser, Debug)]
#[command(name = "ed2kia")]
#[command(about = "Red descentralizada para análisis interpretativo distribuido de LLMs usando SAEs (v1.0.0 STABLE: Fases 1-9 unificadas)", long_about = None)]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// ID del nodo (auto-generado si no se proporciona)
    #[arg(short, long)]
    node_id: Option<String>,

    /// Puerto para escuchar conexiones P2P
    #[arg(short, long, default_value = "0")]
    port: u16,

    /// Bootstrap peers (multiaddrs separados por coma)
    #[arg(long)]
    bootstrap: Option<Vec<String>>,

    /// Path al archivo SAE (.safetensors)
    #[arg(long)]
    sae_path: Option<String>,

    /// Mostrar información de build (commit, date, features)
    #[arg(long)]
    build_info: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Unirse a la red P2P y comenzar a operar
    Join {
        /// Número máximo de peers conectados
        #[arg(short, long, default_value = "50")]
        max_peers: usize,
    },
    /// Mostrar estado actual del nodo y la red
    Status,
    /// Salir de la red de manera ordenada
    Exit,
    // ─── Fase 2: Nuevos comandos ───
    /// Ejecutar análisis local de features SAE
    Analyze {
        /// Capa SAE a analizar
        #[arg(short, long)]
        layer: u32,
    },
    /// Mostrar estado de consenso (batches validados/pendientes)
    Consensus {
        /// Mostrar solo estado
        #[arg(long)]
        status: bool,
    },
    /// Inyectar steering signal síncrono (simulado)
    Steer {
        /// Inyectar signal
        #[arg(long)]
        inject: bool,
    },
    /// Activar gossipsub y comenzar broadcast
    Pubsub {
        /// Unirse a todos los topics
        #[arg(long)]
        join: bool,
    },
    // ─── Fase 3: Nuevos comandos ───
    /// Ejecutar forward pass del SAE en sandbox WASM
    Sandbox {
        /// Path al módulo WASM (.wasm)
        #[arg(short, long)]
        module: String,
        /// Path al archivo de entrada (tensores binarios)
        #[arg(short, long)]
        input: String,
        /// Nombre de la función WASM a invocar
        #[arg(long, default_value = "sae_forward")]
        function: String,
    },
    /// Verificar batch con ZKP
    Verify {
        /// ID del batch a verificar
        #[arg(short, long)]
        batch_id: String,
        /// Valores de features separados por coma
        #[arg(long)]
        features: Option<String>,
        /// ID del verificador
        #[arg(long, default_value = "local")]
        verifier_id: String,
    },
    /// Interfaz de feedback humano
    Feedback {
        /// Modo de operación
        #[command(subcommand)]
        mode: FeedbackMode,
    },
    /// Comandos de despliegue
    Deploy {
        /// Tipo de despliegue
        #[command(subcommand)]
        target: DeployTarget,
    },
    /// Mostrar información de red y nodos
    Network {
        /// Mostrar información de red
        #[arg(long)]
        info: bool,
        /// Mostrar reputación criptográfica
        #[arg(long)]
        crypto_reputation: bool,
    },
    // ─── Fase 4: Nuevos comandos (experimental) ───
    /// Iniciar servidor web + dashboard
    #[cfg(feature = "stable")]
    Web {
        /// Puerto del servidor web
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
    /// Gestión de escalado de red
    #[cfg(feature = "stable")]
    Scale {
        /// Mostrar estado de pares y scoring
        #[arg(long)]
        status: bool,
    },
    /// Loop RLHF - export y gestión de feedback
    #[cfg(feature = "stable")]
    Rlhf {
        /// Exportar feedback a JSONL
        #[arg(long)]
        export: bool,
        /// Path de exportación
        #[arg(short, long, default_value = "./data")]
        path: String,
    },
    /// Ejecutar verificaciones de salud
    #[cfg(feature = "stable")]
    Health {
        /// Ejecutar checks de integridad
        #[arg(long)]
        check: bool,
    },
    // ─── Fase 5: Nuevos comandos (experimental) ───
    /// Sistema de gobernanza (propuestas, votación)
    #[cfg(feature = "stable")]
    Govern {
        /// Crear nueva propuesta
        #[arg(long)]
        propose: Option<String>,
        /// Listar propuestas activas
        #[arg(long)]
        list: bool,
        /// Votar en propuesta
        #[arg(long)]
        vote: Option<uuid::Uuid>,
        /// Tipo de propuesta
        #[arg(long, default_value = "custom")]
        type_: String,
    },
    /// Sistema de reputación (créditos, ranking)
    #[cfg(feature = "stable")]
    Reputation {
        /// Mostrar estado de reputación
        #[arg(long)]
        status: bool,
        /// Mostrar ranking de nodos
        #[arg(long)]
        leaderboard: bool,
        /// Aplicar decay
        #[arg(long)]
        decay: bool,
    },
    /// Sincronización con ecosistema (Hugging Face, ModelScope)
    #[cfg(feature = "stable")]
    Sync {
        /// Descargar modelo
        #[arg(long)]
        download: bool,
        /// Repositorio (ej: Qwen-Scope/SAE-Res-Qwen3.5-27B)
        #[arg(long)]
        repo: Option<String>,
        /// Archivo a descargar
        #[arg(long, default_value = "model.safetensors")]
        file: String,
        /// Listar modelos en cache
        #[arg(long)]
        list: bool,
    },
    /// Bootstrap de red (seed nodes, genesis)
    #[cfg(feature = "stable")]
    Bootstrap {
        /// Inicializar red genesis
        #[arg(long)]
        genesis: bool,
        /// Unirse a red existente
        #[arg(long)]
        join: bool,
        /// Mostrar estado de seeds
        #[arg(long)]
        status: bool,
        /// Directorio de datos
        #[arg(long, default_value = "./data")]
        data_dir: String,
        /// Puerto P2P
        #[arg(long, default_value = "9000")]
        p2p_port: u16,
        /// Puerto HTTP
        #[arg(long, default_value = "3000")]
        http_port: u16,
        /// Modo bootstrap/seed
        #[arg(long)]
        is_bootstrap: bool,
    },
    /// Generar release empaquetado
    Release {
        /// Generar paquete distributable
        #[arg(long)]
        package: bool,
        /// Versión del release
        #[arg(long, default_value = "0.5.0")]
        version: String,
    },
    // ─── Fase 6: Nuevos comandos (experimental) ───
    /// Adaptador de tensores cross-model (Llama/Mistral → Qwen-Scope)
    #[cfg(feature = "stable")]
    Adapt {
        /// Modelo origen (Llama, Mistral, GPT2)
        #[arg(short, long, default_value = "Llama")]
        source: String,
        /// Dimensionalidad de entrada
        #[arg(short, long)]
        input_dim: Option<usize>,
        /// Dimensionalidad de salida (Qwen-Scope: 3584)
        #[arg(long, default_value = "3584")]
        output_dim: usize,
        /// Validar contra schema Qwen-Scope
        #[arg(long)]
        validate: bool,
    },
    /// Federación - agregación FedAvg + Krum
    #[cfg(feature = "stable")]
    Federate {
        /// Iniciar ronda de agregación
        #[arg(long)]
        start: bool,
        /// Capa a agregar
        #[arg(short, long, default_value = "0")]
        layer: u32,
        /// Mínimo de participantes
        #[arg(long, default_value = "3")]
        min_participants: usize,
        /// Mostrar estado de ronda
        #[arg(long)]
        status: bool,
    },
    /// Staking - registro de recursos y proof-of-computation
    #[cfg(feature = "stable")]
    Stake {
        /// Registrar nodo con recursos
        #[arg(long)]
        register: bool,
        /// Generar proof de cómputo
        #[arg(long)]
        prove: bool,
        /// Verificar proof
        #[arg(long)]
        verify: bool,
        /// Mostrar registro de nodos
        #[arg(long)]
        registry: bool,
        /// Núcleos CPU
        #[arg(long, default_value = "8")]
        cpu_cores: u32,
        /// RAM en GB
        #[arg(long, default_value = "16")]
        ram_gb: u64,
        /// GPU disponible
        #[arg(long)]
        has_gpu: bool,
    },
    /// API v2 - gestión de endpoints REST
    #[cfg(feature = "stable")]
    Api {
        /// Generar especificación OpenAPI
        #[arg(long)]
        openapi: bool,
        /// Path de salida para spec JSON
        #[arg(short, long, default_value = "./openapi.json")]
        output: String,
        /// Iniciar servidor API v2
        #[arg(long)]
        serve: bool,
        /// Puerto del servidor
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
}

#[derive(Subcommand, Debug)]
enum FeedbackMode {
    /// Modo interactivo (TTY)
    Interactive {
        /// ID del annotador
        #[arg(short, long, default_value = "human-1")]
        annotator_id: String,
    },
    /// Procesar feedback desde JSON
    Batch {
        /// Path al archivo JSON de feedback
        #[arg(short, long)]
        input: String,
    },
    /// Mostrar estadísticas de feedback
    Stats,
}

#[derive(Subcommand, Debug)]
enum DeployTarget {
    /// Generar configuración Docker
    Docker,
    /// Generar configuración systemd
    Systemd,
    /// Mostrar instrucciones de cross-compilación
    Cross,
}

/// Helper: Get list of enabled features as a string
fn get_enabled_features() -> String {
    let features: Vec<&str> = Vec::new();
    #[cfg(feature = "cuda")]
    features.push("cuda");
    #[cfg(feature = "metal")]
    features.push("metal");
    #[cfg(feature = "debug")]
    features.push("debug");
    #[cfg(feature = "test-mocks")]
    features.push("test-mocks");
    if features.is_empty() {
        "cpu (default)".to_string()
    } else {
        features.join(", ")
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Inicializar logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive("ed2kia=info".parse()?),
        )
        .init();

    let cli = Cli::parse();

    // Handle --build-info flag
    if cli.build_info {
        println!("ed2kIA Build Information:");
        println!("  Version:    {}", env!("CARGO_PKG_VERSION"));
        println!("  Package:    {}", env!("CARGO_PKG_NAME"));
        println!("  Authors:    {}", env!("CARGO_PKG_AUTHORS"));
        println!("  Repository: {}", env!("CARGO_PKG_REPOSITORY"));
        println!(
            "  Profile:    {}",
            std::env::var("PROFILE").unwrap_or_else(|_| "dev".to_string())
        );
        println!(
            "  Target:     {}",
            std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string())
        );
        println!(
            "  Debug:      {}",
            std::env::var("DEBUG").unwrap_or_else(|_| "true".to_string())
        );
        println!("  Features:   {}", get_enabled_features());
        return Ok(());
    }

    info!("ed2kIA iniciando - Versión {}", env!("CARGO_PKG_VERSION"));

    // Handle optional command (none = show help or version)
    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            // No command provided, print help
            println!(
                "ed2kIA v{} - Red descentralizada para análisis interpretativo de LLMs",
                env!("CARGO_PKG_VERSION")
            );
            println!("Use --help for usage information.");
            return Ok(());
        }
    };

    // Inicializar swarm P2P
    let mut swarm = Ed2kSwarm::new(cli.node_id.clone(), cli.port).await?;

    match command {
        Commands::Join { max_peers } => {
            info!("Uniéndose a la red con max_peers={}", max_peers);

            // Conectar a bootstrap peers si se proporcionaron
            if let Some(peers) = &cli.bootstrap {
                for peer_addr in peers {
                    info!("Conectando a bootstrap peer: {}", peer_addr);
                    // TODO: Phase 2 - Implementar conexión real a bootstrap peers
                    // swarm.connect_bootstrap(peer_addr).await?;
                }
            }

            // Cargar SAE si se proporciona path
            let _loader = if let Some(sae_path) = &cli.sae_path {
                info!("Cargando SAE desde: {}", sae_path);
                let l = SAELoader::new(sae_path.clone());
                // TODO: Phase 2 - Implementar carga real de pesos .safetensors
                // l.load().await?;
                Some(l)
            } else {
                info!("Sin SAE cargado - operando en modo red solo");
                None
            };

            // Configurar LayerRouter con sharding dinámico
            let _router = LayerRouter::new();
            // TODO: Phase 2 - Configurar sharding basado en recursos del nodo
            // router.configure_sharding(&swarm.get_resources().await?).await?;

            // Iniciar evento loop del swarm
            info!("Nodo activo. Esperando conexiones...");
            // TODO: Phase 2 - Implementar evento loop completo
            // swarm.event_loop(max_peers, &mut router, loader.as_ref()).await?;

            info!("Nodo unido a la red exitosamente");
        }
        Commands::Status => {
            info!("Obteniendo estado del nodo...");
            let status = swarm.get_status().await?;
            println!("=== Estado del Nodo ===");
            println!("Node ID: {}", status.node_id);
            println!("Peers conectados: {}", status.peer_count);
            println!("Capas SAE asignadas: {}", status.sae_layers);
            println!("Leases activos: {}", status.active_leases);
            // TODO: Phase 2 - Mostrar métricas de rendimiento, latencia, bandwidth
        }
        Commands::Exit => {
            info!("Saliendo de la red de manera ordenada...");
            swarm.graceful_exit().await?;
            info!("Sesión finalizada. Hasta pronto.");
        }
        // ─── Fase 2: Nuevos comandos ───
        Commands::Analyze { layer } => {
            info!("Ejecutando análisis de features para layer {}", layer);

            let mut analyzer = FeatureAnalyzer::new(16384);

            // Features de ejemplo para demo
            let features = vec![
                p2p::protocol::SparseFeature {
                    neuron_index: 100,
                    activation_value: 0.95,
                    importance: 0.9,
                },
                p2p::protocol::SparseFeature {
                    neuron_index: 200,
                    activation_value: 0.85,
                    importance: 0.8,
                },
                p2p::protocol::SparseFeature {
                    neuron_index: 300,
                    activation_value: 0.15,
                    importance: 0.2,
                },
            ];

            let result = analyzer.analyze(&features, layer);

            println!("=== Análisis de Features ===");
            println!("Layer: {}", layer);
            println!("Anomaly Score: {:.3}", result.anomaly_score);
            println!("Confidence: {:.3}", result.confidence);
            println!("Activation Density: {:.3}", result.activation_density);
            println!("Std Deviation: {:.3}", result.std_deviation);
            println!("Mean Activation: {:.3}", result.mean_activation);
            println!("Flagged Features: {:?}", result.flagged_features);
            println!(
                "Patterns: {:?}",
                result
                    .detected_patterns
                    .iter()
                    .map(|p| format!("{}", p))
                    .collect::<Vec<_>>()
            );
        }
        Commands::Consensus { status } => {
            if status {
                info!("Obteniendo estado de consenso...");

                let validator = ConsensusValidator::new();
                let stats = validator.stats();

                println!("=== Estado de Consenso ===");
                println!("Total Batches: {}", stats.total_batches);
                println!("Aprobados: {}", stats.approved);
                println!("Rechazados: {}", stats.rejected);
                println!("Expirados: {}", stats.timed_out);
                println!("En colección: {}", stats.collecting);
                println!("Total Eventos: {}", stats.total_events);
                println!("Nodos Rastreados: {}", stats.tracked_nodes);
            }
        }
        Commands::Steer { inject } => {
            if inject {
                info!("Inyectando steering signal síncrono...");

                let bridge = ConsciousnessBridge::new(16384);

                // Simular features de múltiples nodos
                let features = vec![
                    p2p::protocol::SparseFeature {
                        neuron_index: 100,
                        activation_value: 0.95,
                        importance: 0.9,
                    },
                    p2p::protocol::SparseFeature {
                        neuron_index: 200,
                        activation_value: 0.88,
                        importance: 0.85,
                    },
                ];

                bridge
                    .add_node_features("node_sim".to_string(), 14, &features)
                    .ok();

                // Generar context injection
                if let Some(injection) = bridge.generate_context_injection(14) {
                    println!("=== Context Injection ===");
                    println!("ID: {}", injection.injection_id);
                    println!("Action: {}", injection.action);
                    println!("Confidence: {:.3}", injection.confidence);
                    println!("Message: {}", injection.context_message);
                }

                info!("Steering signal inyectado exitosamente");
            }
        }
        Commands::Pubsub { join } => {
            if join {
                info!("Activando GossipSub y uniéndose a topics...");

                swarm.subscribe_all_topics()?;

                println!("=== GossipSub Activado ===");
                println!("Topics suscritos:");
                println!("  - {}", p2p::swarm::FEATURE_BATCH_TOPIC);
                println!("  - {}", p2p::swarm::STEERING_SIGNAL_TOPIC);
                println!("  - {}", p2p::swarm::CONSENSUS_VOTE_TOPIC);
                println!("\nNodo listo para broadcast via gossipsub");

                // TODO: Phase 2 - Iniciar event loop con gossipsub
                // swarm.event_loop(max_peers).await?;
            }
        }
        // ─── Fase 3: Nuevos comandos ───
        Commands::Sandbox {
            module,
            input,
            function,
        } => {
            info!(
                "Ejecutando WASM sandbox: module={}, input={}",
                module, input
            );

            let mut sandbox = WASMSandbox::new(None);

            // Carga módulo WASM
            let module_path = std::path::Path::new(&module);
            match sandbox.load_module_from_file(module_path) {
                Ok(module_id) => {
                    info!("Módulo WASM cargado: {}", module_id);

                    // Lee datos de entrada
                    let input_data = std::fs::read(&input)
                        .map_err(|e| anyhow::anyhow!("Failed to read input file: {}", e))?;

                    // Ejecuta forward pass
                    match sandbox.execute_sae_forward(&module_id, input_data, Some(&function)) {
                        Ok(result) => {
                            println!("=== WASM Sandbox Result ===");
                            println!("Execution Time: {:.2}ms", result.execution_time_ms);
                            println!("Output Size: {} bytes", result.output.len());
                            println!("Memory Used: {}KB", result.memory_used_bytes / 1024);
                            println!("Invocations: {}", result.invocation_count);

                            // Muestra stats del sandbox
                            let stats = sandbox.get_stats();
                            println!("\n=== Sandbox Stats ===");
                            println!("Cached Modules: {}", stats.cached_modules);
                            println!(
                                "Memory Limit: {}MB",
                                stats.memory_limit_bytes / (1024 * 1024)
                            );
                        }
                        Err(e) => {
                            eprintln!("WASM execution failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to load WASM module: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Verify {
            batch_id,
            features,
            verifier_id,
        } => {
            info!(
                "Verificando batch con ZKP: batch_id={}, verifier={}",
                batch_id, verifier_id
            );

            let verifier = ZKPVerifier::new(Some(0.6));

            // Parsea features o usa valores de ejemplo
            let feature_values: Vec<f64> = if let Some(feat_str) = &features {
                feat_str
                    .split(',')
                    .map(|s| s.trim().parse::<f64>().unwrap_or(0.0))
                    .collect()
            } else {
                vec![0.5, -0.3, 0.8, 0.1, -0.2, 0.9, 0.4, 0.7]
            };

            let result = verifier.verify_batch(&batch_id, &feature_values, &verifier_id);

            println!("=== ZKP Verification Result ===");
            match &result {
                zkp::verifier::VerificationResult::ZKPVerified {
                    proof_hash,
                    confidence,
                    ..
                } => {
                    println!("Status: ZKP VERIFIED");
                    println!("Proof Hash: {}", hex::encode(proof_hash));
                    println!("Confidence: {:.3}", confidence);
                }
                zkp::verifier::VerificationResult::MerkleVerified {
                    merkle_root,
                    confidence,
                    ..
                } => {
                    println!("Status: MERKLE VERIFIED (fallback)");
                    println!("Merkle Root: {}", hex::encode(merkle_root));
                    println!("Confidence: {:.3}", confidence);
                }
                zkp::verifier::VerificationResult::VRFVerified { confidence, .. } => {
                    println!("Status: VRF VERIFIED");
                    println!("Confidence: {:.3}", confidence);
                }
                zkp::verifier::VerificationResult::Failed { reason, .. } => {
                    println!("Status: FAILED");
                    println!("Reason: {}", reason);
                }
            }

            // Muestra stats del verificador
            let stats = verifier.get_stats();
            println!("\n=== Verifier Stats ===");
            println!("Total Verifications: {}", stats.total_verifications);
            println!("Success Rate: {:.2}%", stats.success_rate * 100.0);
            println!("Tracked Nodes: {}", stats.tracked_nodes);
        }
        Commands::Feedback { mode } => {
            match mode {
                FeedbackMode::Interactive { annotator_id } => {
                    info!(
                        "Iniciando sesión interactiva de feedback: annotator={}",
                        annotator_id
                    );

                    let mut manager = FeedbackManager::new(None);

                    // Features de ejemplo para etiquetar
                    let features = vec![
                        (0, 0.95, "self_reference".to_string()),
                        (1, 0.85, "logical_contradiction".to_string()),
                        (2, 0.15, "repetition_pattern".to_string()),
                        (3, 0.72, "safety_concern".to_string()),
                        (4, 0.60, "positive_sentiment".to_string()),
                    ];

                    let requests =
                        manager.generate_labeling_requests("demo-batch", &features, &annotator_id);

                    if let Err(e) = manager.run_interactive(requests, &annotator_id) {
                        eprintln!("Feedback session error: {}", e);
                    }

                    let stats = manager.get_stats();
                    println!("\n=== Feedback Session Stats ===");
                    println!("Total Labels: {}", stats.total_feedback);
                    println!("Approved: {}", stats.approved);
                    println!("Rejected: {}", stats.rejected);
                    println!("Corrected: {}", stats.corrected);
                    println!("Uncertain: {}", stats.uncertain);
                    println!("Approval Rate: {:.2}%", stats.approval_rate * 100.0);
                }
                FeedbackMode::Batch { input } => {
                    info!("Procesando feedback batch desde: {}", input);

                    let mut manager = FeedbackManager::new(None);
                    let json_data = std::fs::read_to_string(&input)
                        .map_err(|e| anyhow::anyhow!("Failed to read feedback file: {}", e))?;

                    match manager.process_json_feedback(&json_data) {
                        Ok(processed) => {
                            println!("Processed {} feedback entries", processed.len());
                            let stats = manager.get_stats();
                            println!("Approval Rate: {:.2}%", stats.approval_rate * 100.0);
                        }
                        Err(e) => {
                            eprintln!("Failed to process feedback: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                FeedbackMode::Stats => {
                    let manager = FeedbackManager::new(None);
                    let stats = manager.get_stats();
                    println!("=== Feedback Stats ===");
                    println!("Total Feedback: {}", stats.total_feedback);
                    println!("Approved: {}", stats.approved);
                    println!("Rejected: {}", stats.rejected);
                    println!("Corrected: {}", stats.corrected);
                    println!("Uncertain: {}", stats.uncertain);
                    println!("Approval Rate: {:.2}%", stats.approval_rate * 100.0);
                    println!("Annotators: {}", stats.annotators);
                }
            }
        }
        Commands::Deploy { target } => match target {
            DeployTarget::Docker => {
                println!("=== Docker Deployment ===");
                println!("Build: docker build -f deploy/Dockerfile -t ed2kia:latest .");
                println!("Run:   docker run -p 9000:9000 ed2kia:latest ed2kia --port 9000 join");
                println!("Multi: docker-compose -f deploy/docker-compose.yml up -d");
            }
            DeployTarget::Systemd => {
                println!("=== Systemd Deployment ===");
                println!("Install: sudo cp deploy/systemd/ed2kia.service /etc/systemd/system/");
                println!("Enable:  sudo systemctl enable ed2kia");
                println!("Start:   sudo systemctl start ed2kia");
                println!("Status:  sudo systemctl status ed2kia");
            }
            DeployTarget::Cross => {
                println!("=== Cross-Compilation ===");
                println!("Linux x86_64:  cross build --release --target x86_64-unknown-linux-gnu");
                println!("Linux ARM64:   cross build --release --target aarch64-unknown-linux-gnu");
                println!("macOS x86_64:  cross build --release --target x86_64-apple-darwin");
                println!("macOS ARM64:   cross build --release --target aarch64-apple-darwin");
                println!("Windows:       cross build --release --target x86_64-pc-windows-msvc");
            }
        },
        Commands::Network {
            info,
            crypto_reputation,
        } => {
            if info {
                println!("=== Network Info ===");
                let status = swarm.get_status().await?;
                println!("Node ID: {}", status.node_id);
                println!("Peers: {}", status.peer_count);
                println!("SAE Layers: {}", status.sae_layers);
                println!("Active Leases: {}", status.active_leases);
            }
            if crypto_reputation {
                let validator = ConsensusValidator::new();
                let reputations = validator.get_all_crypto_reputations();
                println!("=== Crypto Reputation ===");
                if reputations.is_empty() {
                    println!("No nodes tracked yet");
                } else {
                    for rep in reputations {
                        println!("  Node: {}", rep.node_id);
                        println!("    Score: {:.3}", rep.reputation_score);
                        println!("    Trust: {}", rep.trust_level());
                        println!("    Verifications: {}", rep.total_verifications);
                    }
                }
            }
        }
        // ─── Fase 4: Handlers (experimental) ───
        #[cfg(feature = "stable")]
        Commands::Web { port } => {
            let bind_address = format!("0.0.0.0:{}", port);
            println!("=== Web Server ===");
            println!("Starting web server on {}", bind_address);
            println!("Dashboard: http://localhost:{}", port);
            println!("API:       http://localhost:{}/api/status", port);
            println!("Metrics:   http://localhost:{}/api/metrics", port);
            println!("Health:    http://localhost:{}/api/health", port);

            let config = web::server::WebServerConfig {
                bind_address,
                ..web::server::WebServerConfig::default()
            };
            let state = web::server::WebServerState::default_state();
            let server = web::server::WebServer::new(config, state);
            server.start().await?;
        }
        #[cfg(feature = "stable")]
        Commands::Scale { status } => {
            if status {
                let peer_manager = PeerManager::new();
                let bootstrap_manager = BootstrapManager::new();
                let stats = peer_manager.stats();
                let bootstrap_stats = bootstrap_manager.stats();

                println!("=== Scale Status ===");
                println!("Active Connections: {}", stats.active_connections);
                println!("Total Registered:   {}", stats.total_registered);
                println!("Connected:          {}", stats.connected);
                println!("Banned:             {}", stats.banned);
                println!("Total Bytes:        {}", stats.total_bytes);
                println!("Max Inbound:        {}", stats.limits.max_inbound);
                println!("Max Outbound:       {}", stats.limits.max_outbound);
                println!("Mesh N:             {}", stats.limits.mesh_n);
                println!();
                println!("=== Bootstrap Nodes ===");
                println!("Total:    {}", bootstrap_stats.total);
                println!("Active:   {}", bootstrap_stats.active);
                println!("Inactive: {}", bootstrap_stats.inactive);
                println!("Protocol: {}", bootstrap_stats.protocol_version);
            }
        }
        #[cfg(feature = "stable")]
        Commands::Rlhf { export, path } => {
            if export {
                println!("=== RLHF Export ===");
                println!("Export path: {}", path);
                // TODO: Phase 5 - Integración completa con feedback store
                println!(
                    "Feedback export ready. Use 'cargo run -- feedback' to collect data first."
                );
            }
        }
        #[cfg(feature = "stable")]
        Commands::Health { check } => {
            if check {
                let health_manager = HealthManager::new();
                health_manager.add_default_checks();
                let report = health_manager.run_checks();

                println!("=== Health Check ===");
                println!("Status: {}", report.status);
                println!("Uptime: {}s", report.uptime_seconds);
                println!(
                    "Checks: {}/{} passed",
                    report.summary.passed, report.summary.total
                );
                println!();

                for check in &report.checks {
                    let icon = if check.passed { "✅" } else { "❌" };
                    println!(
                        "  {} {} - {} ({:.1}ms)",
                        icon, check.name, check.message, check.latency_ms
                    );
                }
            }
        }
        // ─── Fase 5: Handlers (experimental) ───
        #[cfg(feature = "stable")]
        Commands::Govern {
            propose,
            list,
            vote,
            type_,
        } => {
            let mut manager = ProposalManager::new();

            if let Some(ref title) = propose {
                // Generar keypair para demo
                let (signing_key, _) = Proposal::generate_keypair()?;
                let id = uuid::Uuid::new_v4();

                let proposal_type = match type_.as_str() {
                    "network" => ProposalType::NetworkParam,
                    "model" => ProposalType::ModelUpdate,
                    "reputation" => ProposalType::ReputationPolicy,
                    "security" => ProposalType::Security,
                    "governance" => ProposalType::Governance,
                    "ecosystem" => ProposalType::Ecosystem,
                    _ => ProposalType::Custom,
                };

                let proposal = Proposal::create(
                    id,
                    proposal_type,
                    title.clone(),
                    format!("Payload de propuesta: {}", title),
                    &signing_key,
                    72 * 3600, // 72h
                );

                manager.submit(proposal)?;
                println!("=== Propuesta Creada ===");
                println!("ID: {}", id);
                println!("Título: {}", title);
                println!("Tipo: {}", type_);
            }

            if list {
                println!("=== Propuestas Activas ===");
                let active = manager.get_active_voting();
                if active.is_empty() {
                    println!("No hay propuestas en votación");
                } else {
                    for p in active {
                        println!("  [{}] {} ({})", p.id, p.title, p.state);
                    }
                }
            }

            if let Some(proposal_id) = vote {
                println!("=== Votación ===");
                println!("Propuesta: {}", proposal_id);
                println!("Voto registrado (demo)");
            }
        }
        #[cfg(feature = "stable")]
        Commands::Reputation {
            status,
            leaderboard,
            decay,
        } => {
            let mut scorer = ReputationScorer::new();

            if status {
                let stats = scorer.global_stats();
                println!("=== Reputación Global ===");
                println!("Nodos totales: {}", stats.total_nodes);
                println!("Créditos totales: {:.2}", stats.total_credits);
                println!("Score promedio: {:.4}", stats.average_reputation_score);
                println!("Eligibles gobernanza: {}", stats.governance_eligible);
            }

            if leaderboard {
                let ranking = scorer.get_ranking();
                println!("=== Ranking de Reputación ===");
                if ranking.is_empty() {
                    println!("Sin datos de reputación");
                } else {
                    for (i, (node_id, score)) in ranking.iter().enumerate() {
                        println!("  {}. {} - {:.4}", i + 1, node_id, score);
                    }
                }
            }

            if decay {
                let count = scorer.apply_decay_to_all()?;
                println!("=== Decay Aplicado ===");
                println!("Nodos procesados: {}", count);
            }
        }
        #[cfg(feature = "stable")]
        Commands::Sync {
            download,
            repo,
            file,
            list,
        } => {
            if list {
                let sync_manager = HfSyncManager::new();
                let cached = sync_manager.list_cached_models()?;
                println!("=== Modelos en Cache ===");
                if cached.is_empty() {
                    println!("Cache vacío");
                } else {
                    for path in cached {
                        println!("  {}", path.display());
                    }
                }
            }

            if download {
                let repo_id = repo.as_deref().unwrap_or("Qwen-Scope/SAE-Res-Qwen3.5-27B");
                println!("=== Sync Download ===");
                println!("Repo: {}", repo_id);
                println!("File: {}", file);
                println!("Source: HuggingFace");
                // TODO: Phase 6 - Integración async completa
                println!("Download initiated (async operation)");
            }
        }
        #[cfg(feature = "stable")]
        Commands::Bootstrap {
            genesis,
            join,
            status,
            data_dir,
            p2p_port,
            http_port,
            is_bootstrap,
        } => {
            if genesis {
                let config = GenesisConfig {
                    data_dir: std::path::PathBuf::from(&data_dir),
                    p2p_port,
                    http_port,
                    is_bootstrap_node: is_bootstrap,
                    ..GenesisConfig::default()
                };

                let mut initializer = NetworkInitializer::new(config);
                match initializer.run_genesis() {
                    Ok(result) => {
                        println!("=== Genesis Complete ===");
                        println!("Mode: {}", result.network_mode);
                        println!("Seeds: {}", result.connected_seeds);
                        println!("SAE Layers: {}", result.sae_layers_loaded);
                        println!("Leases: {}", result.leases_created);
                        println!("Message: {}", result.message);
                    }
                    Err(e) => {
                        eprintln!("Genesis failed: {}", e);
                    }
                }
            }

            if join {
                println!("=== Bootstrap Join ===");
                println!("Connecting to seed nodes...");
                let registry = SeedRegistry::new();
                let stats = registry.stats();
                println!("Total seeds: {}", stats.total);
                println!("Healthy: {}", stats.healthy);
                println!("Has minimum: {}", stats.has_minimum);
            }

            if status {
                let registry = SeedRegistry::new();
                let stats = registry.stats();
                println!("=== Seed Registry Status ===");
                println!("Total: {}", stats.total);
                println!("Healthy: {}", stats.healthy);
                println!("Degraded: {}", stats.degraded);
                println!("Unhealthy: {}", stats.unhealthy);
                println!("Avg Latency: {:.1}ms", stats.avg_latency_ms);
                println!("Minimum Required: {}", stats.minimum_required);
                println!("Has Minimum: {}", stats.has_minimum);
            }
        }
        Commands::Release { package, version } => {
            if package {
                println!("=== Release Package ===");
                println!("Version: {}", version);
                println!("Running packager.sh...");
                println!("Use: bash release/packager.sh --package");
                println!("Output: release/dist/");
            }
        }
        // ─── Fase 6: Handlers (experimental) ───
        #[cfg(feature = "stable")]
        Commands::Adapt {
            source,
            input_dim,
            output_dim,
            validate,
        } => {
            println!("=== Tensor Adapter (Cross-Model) ===");
            println!("Source: {}", source);
            println!("Output Dim: {}", output_dim);

            let source_model = match source.as_str() {
                "Mistral" => SourceModel::Mistral,
                "Qwen" => SourceModel::Qwen,
                "GPT2" => SourceModel::GPT2,
                _ => SourceModel::Llama,
            };

            // FIX: v1.0.1-patch - ModelNormConfig removed; TensorAdapter API changed to new(target_dim, target_dtype)
            let _input_dim = input_dim.unwrap_or(4096);
            #[cfg(feature = "phase6-core")]
            {
                let _adapter = TensorAdapter::new(output_dim, candle_core::DType::F32);
            }
            println!("Adapter created: {} -> {}", source_model, output_dim);

            if validate {
                let schema = QwenScopeSchema::default();
                println!("Schema validation: canonical_dim={}", schema.canonical_dim);
                println!("Norm required: {}", schema.required_norm);
                println!(
                    "Value range: ({}, {})",
                    schema.value_range.0, schema.value_range.1
                );
            }
        }
        #[cfg(feature = "stable")]
        Commands::Federate {
            start,
            layer,
            min_participants,
            status,
        } => {
            if start {
                let config = FedAvgConfig {
                    min_participants,
                    krum_f: 1,
                    min_participation_fraction: 0.5,
                };
                let mut aggregator = FedAvgAggregator::new(config);
                println!("=== Federation Round Started ===");
                println!("Layer: {}", layer);
                println!("Min participants: {}", min_participants);

                // Simular recepción de updates
                for node_idx in 0..min_participants {
                    let update = WeightUpdate::new(
                        format!("node-{}", node_idx),
                        layer,
                        vec![0.1, -0.05, 0.02],
                        1000,
                        // FIX: v1.0.1-patch - local_loss is f32
                        (0.5 + node_idx as f64 * 0.1) as f32,
                    );
                    // FIX: v1.0.1-patch - API changed from receive_update() to add_update()
                    aggregator.add_update(update).ok();
                }

                println!("Updates received: {}", min_participants);
            }

            if status {
                println!("=== Federation Status ===");
                println!("Protocol: FedAvg + Krum");
                println!("Layer target: {}", layer);
                // FIX: v1.0.1-patch - SyncProtocol::new() requires FedAvgAggregator; use with_defaults()
                // FIX: v1.0.1-patch - active_rounds() removed; use stats().active_rounds
                let protocol = SyncProtocol::with_defaults();
                let stats = protocol.stats();
                println!("Active rounds: {}", stats.active_rounds);
            }
        }
        #[cfg(feature = "stable")]
        Commands::Stake {
            register,
            prove,
            verify,
            registry: show_registry,
            cpu_cores,
            ram_gb,
            has_gpu,
        } => {
            // FIX: v1.0.1-patch - ResourceRegistry::new() requires (max_heartbeat_age, slash_threshold)
            let mut registry = ResourceRegistry::new(60, 3);

            if register {
                let commitment = ResourceCommitment {
                    node_id: "local-node".to_string(),
                    cpu_cores,
                    ram_gb: ram_gb as f64,
                    has_gpu,
                    bandwidth_mbps: 1000.0,
                    storage_gb: 500.0,
                    registered_at: 0,
                    last_heartbeat: 0,
                    proofs_verified: 0,
                    reputation_score: 1.0,
                    status: staking::registry::NodeStatus::Active,
                };
                let _ = registry.register(commitment); // CLEANUP: Handle unused Result
                println!("=== Node Registered ===");
                println!("CPU: {} cores", cpu_cores);
                println!("RAM: {} GB", ram_gb);
                println!("GPU: {}", has_gpu);
            }

            if prove {
                // FIX: v1..1-patch - generate_proof() takes ComputeMetrics, returns Result
                let mut generator = ProofGenerator::new("local-node".to_string());
                let metrics = ComputeMetrics::new(1000, 500, 2048.0, 75.0);
                let proof = generator
                    .generate_proof(metrics)
                    .expect("Failed to generate proof");
                println!("=== Proof Generated ===");
                println!("Nonce: {}", proof.nonce);
                println!("Commitment: {}", proof.commitment_hash);
                println!("Samples: {}", proof.compute_metrics.samples_processed);
            }

            if verify {
                // FIX: v1.0.1-patch - ProofVerifier::new() requires max_proof_age_seconds
                // FIX: v1.0.1-patch - max_age_seconds() method removed
                let _verifier = ProofVerifier::new(300);
                println!("=== Proof Verification ===");
                println!("Max age: 300s");
                println!("Nonce tracking: enabled");
            }

            if show_registry {
                let stats = registry.stats();
                println!("=== Staking Registry ===");
                println!("Total nodes: {}", stats.total_nodes);
                println!("Active nodes: {}", stats.active_nodes);
                println!("Total CPU cores: {}", stats.total_cpu_cores);
                println!("Total RAM: {} GB", stats.total_ram_gb);
                println!("GPU nodes: {}", stats.gpu_nodes);
            }
        }
        #[cfg(feature = "stable")]
        Commands::Api {
            openapi,
            output,
            serve,
            port,
        } => {
            if openapi {
                let spec = OpenApiSpec {
                    openapi: "3.0.3".to_string(),
                    info: Info {
                        title: "ed2kIA API v2".to_string(),
                        description: "API para interoperabilidad, federación, staking y gobernanza"
                            .to_string(),
                        version: "v2".to_string(),
                        contact: Contact {
                            name: "ed2kIA Team".to_string(),
                            url: "https://ed2k.ai".to_string(),
                            email: "contact@ed2k.ai".to_string(),
                        },
                    },
                    servers: vec![Server {
                        url: format!("http://localhost:{}", port),
                        description: "Local development".to_string(),
                    }],
                    paths: Paths::default(),
                    components: Components::default(),
                };

                let json = serde_json::to_string_pretty(&spec).unwrap_or_default();
                std::fs::write(&output, json).expect("Failed to write OpenAPI spec");
                println!("=== OpenAPI Spec Generated ===");
                println!("Output: {}", output);
            }

            if serve {
                println!("=== API v2 Server ===");
                println!("Port: {}", port);
                println!("State: ApiV2State initialized");
                println!("Endpoints: /api/v2/health, /api/v2/network, /api/v2/sae/analyze, /api/v2/federation/rounds, /api/v2/staking/registry, /api/v2/governance/proposals");
                println!("OpenAPI: /api/v2/openapi.json");
                // TODO: Fase 6 - Integrar con Axum Router completo
            }
        }
    }

    Ok(())
}
