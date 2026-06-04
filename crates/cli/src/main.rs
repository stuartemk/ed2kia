//! ed2k-cli — Command-Line Interface
//!
//! Node operations, configuration, and monitoring CLI.

use clap::Parser;

#[derive(Parser)]
#[command(name = "ed2k-cli")]
#[command(about = "ed2kIA node operations CLI")]
struct Cli {
    /// Node configuration file
    #[arg(short, long)]
    config: Option<String>,

    /// Log level
    #[arg(short, long, default_value = "info")]
    verbose: String,
}

fn main() {
    let cli = Cli::parse();
    println!("ed2k-cli v{}", env!("CARGO_PKG_VERSION"));
    println!("Config: {:?}", cli.config);
    println!("Log level: {}", cli.verbose);
}
