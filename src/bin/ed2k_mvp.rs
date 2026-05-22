//! ed2k_mvp — Binario de ejecución rápida para MVP local.
//!
//! Demuestra el ciclo completo del Kernel Estuardiano:
//! 3 nodos → SAE payloads → SCT Guard → BFT Consensus → Resultados
//!
//! Uso:
//!   cargo run --bin ed2k_mvp --features "v2.1-mvp-simulation" -- --dry-run --verbose
//!   cargo run --bin ed2k_mvp --features "v2.1-mvp-simulation" -- --output-json

use clap::Parser;
use serde::Serialize;
use std::fs;
use std::time::Instant;

/// ed2kIA MVP — End-to-End Local Simulation
#[derive(Parser, Debug)]
#[command(name = "ed2k_mvp")]
#[command(about = "ed2kIA MVP: End-to-End Local Testnet Simulation")]
#[command(
    long_about = "ed2kIA MVP — End-to-End Local Simulation (La Chispa)\n\nDemuestra el ciclo completo del Kernel Estuardiano:\n3 nodos → SAE payloads → SCT Guard → BFT Consensus → Resultados\n\nLey 2 (Reconocimiento del Error): SCT Hard Reject cuando Z < 0\nLey 3 (Cero desperdicio): Simulación ligera, logs deterministas"
)]
struct Cli {
    /// Dry-run mode (simulate without network binding)
    #[arg(long, default_value_t = true)]
    dry_run: bool,

    /// Verbose output
    #[arg(long, short = 'v')]
    verbose: bool,

    /// Export telemetry to JSON file
    #[arg(long, value_name = "FILE", default_value = "mvp-telemetry.json")]
    output_json: String,
}

/// Telemetry export structure.
#[derive(Debug, Serialize)]
struct MvpTelemetry {
    /// Simulation mode.
    dry_run: bool,
    /// Total duration in ms.
    total_duration_ms: f64,
    /// Simulation success.
    success: bool,
    /// Timestamp ISO 8601.
    timestamp: String,
    /// Consensus metrics.
    consensus: ConsensusSummary,
    /// Node summary.
    nodes: Vec<NodeSummary>,
}

#[derive(Debug, Serialize)]
struct ConsensusSummary {
    total_payloads: usize,
    approved_count: usize,
    rejected_count: usize,
    bft_converged: bool,
    total_latency_ms: f64,
}

#[derive(Debug, Serialize)]
struct NodeSummary {
    id: String,
    address: String,
    profile: String,
    final_state: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let start = Instant::now();

    // Print header
    println!("\x1b[36m");
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║              ed2kIA MVP — La Chispa                     ║");
    println!("║         End-to-End Local Testnet Simulation             ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!("\x1b[0m");

    if cli.verbose {
        println!(
            "\n[CONFIG] dry_run={}, verbose={}",
            cli.dry_run, cli.verbose
        );
        println!("[CONFIG] output_json={}", cli.output_json);
    }

    // Build and run testnet
    let mut testnet = ed2kia::mvp::local_testnet::LocalTestnet::new(cli.dry_run);
    testnet.setup_default_cluster();

    let result = testnet.run().await.map_err(|e| {
        println!("\n\x1b[31m[ERROR] Simulation failed: {}\x1b[0m", e);
        e
    })?;

    let total_duration = start.elapsed().as_secs_f64() * 1000.0;

    let consensus = &result.consensus_metrics;

    // Export telemetry if requested
    if cli.output_json != "-" {
        let telemetry = MvpTelemetry {
            dry_run: result.dry_run,
            total_duration_ms: (total_duration * 100.0).round() / 100.0,
            success: result.success,
            timestamp: result.timestamp.clone(),
            consensus: ConsensusSummary {
                total_payloads: consensus.total_payloads,
                approved_count: consensus.approved_count,
                rejected_count: consensus.rejected_count,
                bft_converged: consensus.bft_result.is_some(),
                total_latency_ms: (consensus.total_latency_ms * 100.0).round() / 100.0,
            },
            nodes: result
                .nodes
                .iter()
                .map(|n| NodeSummary {
                    id: n.id.clone(),
                    address: n.address.clone(),
                    profile: format!("{}", n.profile),
                    final_state: format!("{}", n.state),
                })
                .collect(),
        };

        let json = serde_json::to_string_pretty(&telemetry)?;
        fs::write(&cli.output_json, &json)?;
        println!("\n[✓] Telemetry exported to {}", cli.output_json);
    }

    // Final status
    println!();
    if result.success {
        println!("\x1b[32m✅ MVP SIMULATION PASSED\x1b[0m");
        println!(
            "   Duration: {:.1}ms | Approved: {} | Rejected: {} | BFT: {}",
            total_duration,
            consensus.approved_count,
            consensus.rejected_count,
            if consensus.bft_result.is_some() {
                "CONVERGED"
            } else {
                "FAILED"
            }
        );
    } else {
        println!("\x1b[31m❌ MVP SIMULATION FAILED\x1b[0m");
        println!("   Duration: {:.1}ms", total_duration);
    }

    Ok(())
}
