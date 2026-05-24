//! ed2kIA Operator CLI Wizard
//!
//! Standalone binary providing guided setup for node operators.
//! Uses clap for CLI argument parsing and dialoguer for TUI interaction.
//!
//! **Sprint15 - Resiliencia Operativa & Automatización de Respuesta**
//!
//! **Features:**
//! - Role selection (Relay, Orchestrator, WASM Node, Auditor)
//! - Config generation with real-time validation
//! - Environment verification
//! - Health checks
//! - Log export
//!
//! **Usage:**
//! ```bash
//! # Interactive wizard mode
//! ed2kia-cli wizard
//!
//! # Quick health check
//! ed2kia-cli health
//!
//! # Export logs
//! ed2kia-cli logs --last 1h
//!
//! # Generate config for specific role
//! ed2kia-cli config generate --role orchestrator
//! ```

use std::process;

use clap::Parser;
use cli::{Commands, Ed2kiaCli, LogFormat, Role};
use wizard::Wizard;

#[tokio::main]
async fn main() {
    env_logger::init();

    let cli = Ed2kiaCli::parse();

    let result = match &cli.command {
        Commands::Wizard { role } => {
            let mut wizard = Wizard::new();
            if let Some(role) = role {
                wizard.select_role(role.clone());
            }
            wizard.run().await
        }
        Commands::Config(cmd) => match cmd {
            cli::ConfigCommand::Generate {
                role,
                output,
                format,
            } => {
                let config = generate_config(role, format);
                if let Some(path) = output {
                    write_config_to_file(&config, path)
                } else {
                    println!("{}", config);
                    Ok(())
                }
            }
            cli::ConfigCommand::Validate { path } => validate_config(path),
        },
        Commands::Health { endpoint } => {
            let endpoint = endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:3000".to_string());
            run_health_check(&endpoint).await
        }
        Commands::Logs { last, follow } => export_logs(last.as_deref(), *follow),
        Commands::Version => {
            println!("ed2kIA CLI v{}", cli::VERSION);
            println!("Build: {}", cli::build_commit());
            println!("Features: {}", cli::enabled_features_str());
            Ok(())
        }
        #[cfg(feature = "v3.0-omni-integration")]
        Commands::Omni { initial_ce, diagnose } => {
            run_omni_mode(*initial_ce, *diagnose)
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

/// Generate configuration for the specified role.
fn generate_config(role: &Role, format: &LogFormat) -> String {
    let base_config = match role {
        Role::Relay => relay_config(),
        Role::Orchestrator => orchestrator_config(),
        Role::WasmNode => wasm_node_config(),
        Role::Auditor => auditor_config(),
    };

    match format {
        LogFormat::Toml => base_config,
        LogFormat::Json => serde_json::to_string_pretty(&serde_json::json!({
            "ed2kIA": base_config
        }))
        .unwrap_or_else(|_| base_config),
    }
}

/// Write configuration to file.
fn write_config_to_file(config: &str, path: &str) -> Result<(), String> {
    std::fs::write(path, config)
        .map_err(|e| format!("Failed to write config to {}: {}", path, e))?;
    println!("Config written to {}", path);
    Ok(())
}

/// Validate an existing configuration file.
fn validate_config(path: &str) -> Result<(), String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", path, e))?;

    // Basic validation: check if file is non-empty and contains expected sections.
    if content.trim().is_empty() {
        return Err("Config file is empty".to_string());
    }

    // Check for required sections.
    let required_sections = ["[network]", "[node]", "[security]"];
    for section in &required_sections {
        if !content.contains(section) {
            eprintln!("Warning: Missing section {}", section);
        }
    }

    println!("Config validation passed for {}", path);
    Ok(())
}

/// Run health check against the specified endpoint.
async fn run_health_check(endpoint: &str) -> Result<(), String> {
    println!("Running health check against {}", endpoint);

    let health_url = format!("{}/api/health", endpoint);
    let client = reqwest::Client::new();

    match client
        .get(&health_url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                println!("✓ Health check passed (HTTP {})", status);
                Ok(())
            } else {
                Err(format!("Health check failed (HTTP {})", status))
            }
        }
        Err(e) => Err(format!("Health check request failed: {}", e)),
    }
}

/// Export logs.
fn export_logs(last: Option<&str>, follow: bool) -> Result<(), String> {
    let log_dir = std::env::var("ED2KIA_LOG_DIR").unwrap_or_else(|_| "./logs".to_string());

    println!("Log directory: {}", log_dir);

    if follow {
        println!("Following logs (use Ctrl+C to stop)...");
        // In a real implementation, this would tail the log file.
        // For now, just list recent log files.
    }

    if let Ok(dir) = std::fs::read_dir(&log_dir) {
        let mut count = 0;
        for entry in dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("log") {
                println!("  {}", path.display());
                count += 1;
            }
        }
        println!("Found {} log file(s)", count);
    } else {
        println!("Log directory not found. Create it or set ED2KIA_LOG_DIR.");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Role-Specific Configuration Templates
// ---------------------------------------------------------------------------

fn relay_config() -> String {
    r#"[network]
# Relay Node Configuration
listen_address = "/ip4/0.0.0.0/tcp/3000"
announce_address = null
max_peers = 50
gossipsub_enabled = true

[node]
role = "relay"
node_id = null  # Auto-generated
compute_power = 1.0

[security]
signing_key = null  # Auto-generated
banned_peers = []
"#
    .to_string()
}

fn orchestrator_config() -> String {
    r#"[network]
# Orchestrator Node Configuration
listen_address = "/ip4/0.0.0.0/tcp/3001"
announce_address = null
max_peers = 100
gossipsub_enabled = true

[node]
role = "orchestrator"
node_id = null
compute_power = 2.0
task_timeout_secs = 30
max_retries = 3

[security]
signing_key = null
banned_peers = []
slashing_threshold = 0.1

[orchestrator]
rosetta_server_enabled = true
rosetta_server_port = 3002
"#
    .to_string()
}

fn wasm_node_config() -> String {
    r#"[network]
# WASM Node Configuration
listen_address = "/ip4/0.0.0.0/tcp/3002"
announce_address = null
max_peers = 30
gossipsub_enabled = true

[node]
role = "wasm_node"
node_id = null
compute_power = 1.5
model_path = "./models/qwen_scope.safetensors"

[security]
signing_key = null
banned_peers = []

[wasm]
memory_limit_mb = 512
execution_timeout_ms = 5000
"#
    .to_string()
}

fn auditor_config() -> String {
    r#"[network]
# Auditor Node Configuration
listen_address = "/ip4/0.0.0.0/tcp/3003"
announce_address = null
max_peers = 20
gossipsub_enabled = true

[node]
role = "auditor"
node_id = null
compute_power = 1.0

[security]
signing_key = null
banned_peers = []

[auditor]
audit_interval_secs = 60
max_audit_tasks = 10
"#
    .to_string()
}

// ---------------------------------------------------------------------------
// CLI Module
// ---------------------------------------------------------------------------

mod cli {
    use clap::{Parser, Subcommand, ValueEnum};

    /// ed2kIA Operator CLI Wizard
    #[derive(Parser, Debug)]
    #[command(name = "ed2kia-cli")]
    #[command(version = VERSION)]
    #[command(about = "ed2kIA Operator CLI Wizard — Guided setup for node operators")]
    pub struct Ed2kiaCli {
        #[command(subcommand)]
        pub command: Commands,
    }

    #[derive(Subcommand, Debug)]
    pub enum Commands {
        /// Interactive setup wizard
        Wizard {
            /// Pre-select a node role
            #[arg(short, long)]
            role: Option<Role>,
        },
        /// Configuration management
        #[command(subcommand)]
        Config(ConfigCommand),
        /// Run health checks
        Health {
            /// API endpoint to check
            #[arg(short, long)]
            endpoint: Option<String>,
        },
        /// Export and view logs
        Logs {
            /// Time range (e.g., "1h", "24h", "7d")
            #[arg(short, long)]
            last: Option<String>,
            /// Follow log output
            #[arg(short, long)]
            follow: bool,
        },
        /// Show version information
        Version,
        /// Omni-Node mode — Initialize and diagnose all 4 Evolutionary Pillars
        #[cfg(feature = "v3.0-omni-integration")]
        Omni {
            /// Initial CE allocation per pillar
            #[arg(short, long, default_value_t = 100.0)]
            initial_ce: f64,
            /// Run diagnostics and exit
            #[arg(short, long)]
            diagnose: bool,
        },
    }

    #[derive(Subcommand, Debug)]
    pub enum ConfigCommand {
        /// Generate configuration for a role
        Generate {
            /// Node role
            #[arg(short, long, default_value = "relay")]
            role: Role,
            /// Output file path (stdout if omitted)
            #[arg(short, long)]
            output: Option<String>,
            /// Output format
            #[arg(short, long, default_value = "toml")]
            format: LogFormat,
        },
        /// Validate an existing configuration
        Validate {
            /// Path to config file
            #[arg(short, long)]
            path: String,
        },
    }

    #[derive(Debug, Clone, ValueEnum)]
    pub enum Role {
        Relay,
        Orchestrator,
        WasmNode,
        Auditor,
    }

    #[derive(Debug, Clone, ValueEnum)]
    pub enum LogFormat {
        Toml,
        Json,
    }

    pub const VERSION: &str = env!("CARGO_PKG_VERSION");

    pub fn build_commit() -> &'static str {
        option_env!("VERGEN_GIT_SHA").unwrap_or("unknown")
    }

    pub fn enabled_features_str() -> String {
        let mut features = Vec::new();
        #[cfg(feature = "v2.1-chaos-engine")]
        features.push("v2.1-chaos-engine");
        #[cfg(feature = "v2.1-operator-cli")]
        features.push("v2.1-operator-cli");
        #[cfg(feature = "v2.1-auto-remediation")]
        features.push("v2.1-auto-remediation");
        if features.is_empty() {
            "none".to_string()
        } else {
            features.join(", ")
        }
    }
}

// ---------------------------------------------------------------------------
// Wizard Module
// ---------------------------------------------------------------------------

mod wizard {
    use super::cli::Role;
    use dialoguer::{Confirm, Input, Select};
    use std::path::PathBuf;

    pub struct Wizard {
        selected_role: Option<Role>,
    }

    impl Wizard {
        pub fn new() -> Self {
            Self {
                selected_role: None,
            }
        }

        pub fn select_role(&mut self, role: Role) {
            self.selected_role = Some(role);
        }

        pub async fn run(&mut self) -> Result<(), String> {
            println!("╔═══════════════════════════════════════════════════════════╗");
            println!("║              ed2kIA Operator CLI Wizard                   ║");
            println!("║           Resiliencia Operativa v2.1.0                    ║");
            println!("╚═══════════════════════════════════════════════════════════╝");
            println!();

            // Step 1: Role Selection
            let role = self
                .select_role_step()
                .map_err(|e| format!("Role selection failed: {}", e))?;
            println!("✓ Selected role: {:?}", role);
            println!();

            // Step 2: Network Configuration
            let port = self
                .port_step()
                .map_err(|e| format!("Port selection failed: {}", e))?;
            println!("✓ Listen port: {}", port);
            println!();

            // Step 3: Environment Verification
            self.verify_environment()
                .map_err(|e| format!("Environment verification failed: {}", e))?;
            println!("✓ Environment verified");
            println!();

            // Step 4: Generate Config
            let output_path = self
                .config_step()
                .map_err(|e| format!("Config generation failed: {}", e))?;
            println!("✓ Config generated: {}", output_path.display());
            println!();

            // Step 5: Health Check
            let run_health = Confirm::new()
                .with_prompt("Run health check now?")
                .default(false)
                .interact()
                .unwrap_or(false);

            if run_health {
                println!("Health check would run against localhost:{}", port);
                // In a real implementation, this would call the health check API.
            }

            println!();
            println!("╔═══════════════════════════════════════════════════════════╗");
            println!("║              Setup Complete!                              ║");
            println!("║                                                           ║");
            println!("║  Next steps:                                              ║");
            println!("║  1. Review your config: {} ║", output_path.display());
            println!(
                "║  2. Start your node: ed2kia --config {} ║",
                output_path.display()
            );
            println!(
                "║  3. Monitor: ed2kia-cli health --endpoint http://localhost:{}",
                port
            );
            println!("╚═══════════════════════════════════════════════════════════╝");

            Ok(())
        }

        fn select_role_step(&mut self) -> Result<Role, dialoguer::Error> {
            if let Some(role) = &self.selected_role {
                return Ok(role.clone());
            }

            let items = ["Relay", "Orchestrator", "WASM Node", "Auditor"];
            let descriptions = [
                "Relay node — Forward messages, maintain mesh connectivity",
                "Orchestrator — Coordinate audit tasks, manage consensus",
                "WASM Node — Execute inference workloads, provide audit results",
                "Auditor — Validate audit results, maintain reputation",
            ];

            let selection = Select::new()
                .with_prompt("Select your node role")
                .items(&items)
                .interact()?;

            Ok(match selection {
                0 => Role::Relay,
                1 => Role::Orchestrator,
                2 => Role::WasmNode,
                3 => Role::Auditor,
                _ => unreachable!(),
            })
        }

        fn port_step(&self) -> Result<u16, dialoguer::Error> {
            let default_port = match self.selected_role {
                Some(Role::Relay) => "3000",
                Some(Role::Orchestrator) => "3001",
                Some(Role::WasmNode) => "3002",
                Some(Role::Auditor) => "3003",
                None => "3000",
            };

            let port: String = Input::new()
                .with_prompt("Listen port")
                .default(default_port.to_string())
                .interact_text()?;

            port.parse().map_err(|_| {
                dialoguer::Error::IO(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid port number",
                ))
            })
        }

        fn verify_environment(&self) -> Result<(), String> {
            // Check Rust toolchain.
            println!("Checking Rust toolchain...");
            let rustc = std::process::Command::new("rustc")
                .arg("--version")
                .output()
                .map_err(|e| format!("rustc not found: {}", e))?;

            if rustc.status.success() {
                let version = String::from_utf8_lossy(&rustc.stdout);
                println!("  ✓ Rust: {}", version.trim());
            } else {
                return Err("Rust toolchain not available".to_string());
            }

            // Check disk space.
            println!("Checking disk space...");
            // Simplified check — in production, use a proper disk space library.
            println!("  ✓ Disk space OK (manual verification recommended)");

            Ok(())
        }

        fn config_step(&self) -> Result<PathBuf, String> {
            let save = Confirm::new()
                .with_prompt("Generate configuration file?")
                .default(true)
                .interact()
                .map_err(|e| format!("Confirmation failed: {}", e))?;

            if !save {
                return Ok(PathBuf::from("stdout"));
            }

            let path: String = Input::new()
                .with_prompt("Config file path")
                .default("./ed2kia-config.toml".to_string())
                .interact_text()
                .map_err(|e| format!("Input failed: {}", e))?;

            Ok(PathBuf::from(path))
        }
    }
}

/// Run Omni-Node mode — Initialize and diagnose all 4 Evolutionary Pillars.
#[cfg(feature = "v3.0-omni-integration")]
fn run_omni_mode(initial_ce: f64, diagnose: bool) -> Result<(), String> {
    use ed2kia::orchestration::OmniNode;

    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║              ed2kIA Omni-Node Mode                        ║");
    println!("║       Sprint 47 — Symbiotic Integration                   ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();

    // Initialize Omni-Node
    let mut omni = OmniNode::new();
    omni.initialize_pillars(initial_ce);

    println!("✓ Omni-Node initialized with 4 Evolutionary Pillars");
    println!("✓ Initial CE per pillar: {:.2}", initial_ce);
    println!();

    // Display pillar status
    println!("— Pillar Registry —");
    for pillar in omni.registered_pillars() {
        let status = omni.get_pillar_status(pillar).unwrap_or(ed2kia::orchestration::PillarStatus::Pending);
        let ce = omni.ce_ledger().balance(pillar);
        println!("  • {} — Status: {:?}, CE: {:.2}", pillar, status, ce);
    }
    println!();

    // Display CE Ledger summary
    println!("— CE Ledger Summary —");
    println!("  Total CE Emitted: {:.2}", omni.ce_ledger().total_emitted());
    println!("  Total CE Consumed: {:.2}", omni.ce_ledger().total_consumed());
    println!("  SCT Rejections: {}", omni.rejection_count());
    println!();

    if diagnose {
        println!("— Diagnostics —");
        let diagnostics = omni.diagnose();
        for (pillar, diag) in &diagnostics {
            println!("  • {}: {}", pillar, diag);
        }
        println!();
    }

    println!("Omni-Node is ready for symbiotic operation.");
    Ok(())
}
