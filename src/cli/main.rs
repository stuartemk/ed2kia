//! ed2kIA CLI — Sprint 82: Tactical Pivot & Distributed SAE Audit MVP
//!
//! Lightweight CLI for onboarding, model auditing, and credit management.
//! Default model: qwen3.5:2b (2GB RAM minimum).
//!
//! ### Usage
//! ```bash
//! ed2k start --model qwen3.5:2b
//! ed2k audit --prompt "test input"
//! ed2k status
//! ed2k credits
//! ```

use std::fmt;

// ============================================================================
// CLI Commands
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum CliCommand {
    Start { model: String },
    Audit { prompt: String },
    Status,
    Credits,
    Help,
}

impl fmt::Display for CliCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliCommand::Start { model } => write!(f, "start --model {}", model),
            CliCommand::Audit { prompt } => write!(f, "audit --prompt \"{}\"", prompt),
            CliCommand::Status => write!(f, "status"),
            CliCommand::Credits => write!(f, "credits"),
            CliCommand::Help => write!(f, "help"),
        }
    }
}

// ============================================================================
// CLI Parser
// ============================================================================

pub struct CliParser;

impl CliParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse CLI arguments into a command.
    pub fn parse(&self, args: &[String]) -> Result<CliCommand, String> {
        if args.is_empty()
            || args.contains(&"--help".to_string())
            || args.contains(&"-h".to_string())
        {
            return Ok(CliCommand::Help);
        }

        let cmd = &args[0];
        match cmd.as_str() {
            "start" => {
                let model =
                    Self::extract_flag(args, "--model").unwrap_or_else(|| "qwen3.5:2b".to_string());
                Ok(CliCommand::Start { model })
            }
            "audit" => {
                let prompt = Self::extract_flag(args, "--prompt")
                    .ok_or("Missing --prompt for audit command")?;
                Ok(CliCommand::Audit { prompt })
            }
            "status" => Ok(CliCommand::Status),
            "credits" => Ok(CliCommand::Credits),
            unknown => Err(format!(
                "Unknown command: {}. Use --help for usage.",
                unknown
            )),
        }
    }

    fn extract_flag(args: &[String], flag: &str) -> Option<String> {
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            if arg == flag {
                return iter.next().cloned();
            }
        }
        None
    }
}

impl Default for CliParser {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// CLI Runner
// ============================================================================

pub struct CliRunner;

impl CliRunner {
    pub fn new() -> Self {
        Self
    }

    /// Execute a parsed command and return the result string.
    pub fn execute(&self, command: &CliCommand) -> String {
        match command {
            CliCommand::Start { model } => {
                format!("🚀 Starting ed2kIA node with model: {}\n🌍 Joining distributed SAE audit network...", model)
            }
            CliCommand::Audit { prompt } => {
                format!("🔍 Auditing prompt via distributed SAE pipeline: \"{}\"", prompt)
            }
            CliCommand::Status => {
                "📊 Node Status:\n  Model: qwen3.5:2b\n  Pipeline: Active\n  Credits: 0 CE\n  Peers: 0".to_string()
            }
            CliCommand::Credits => {
                "💰 Compute Credits (CE):\n  Balance: 0 CE\n  Earned: 0 CE\n  Spent: 0 CE".to_string()
            }
            CliCommand::Help => {
                "ed2kIA — Decentralized SAE Audit Network for Local LLMs

Usage:
  ed2k start [--model <name>]  Start node (default: qwen3.5:2b)
  ed2k audit --prompt <text>   Audit a prompt via SAE pipeline
  ed2k status                  Show node status
  ed2k credits                 Show compute credit balance
  ed2k help                    Show this help message

Examples:
  ed2k start --model qwen3.5:2b
  ed2k audit --prompt \"Is this output ethically aligned?\"
  ed2k status
"
                .to_string()
            }
        }
    }
}

impl Default for CliRunner {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Run the CLI with the given arguments.
pub fn run(args: &[String]) -> Result<String, String> {
    let parser = CliParser::new();
    let runner = CliRunner::new();
    let command = parser.parse(args)?;
    Ok(runner.execute(&command))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_help_empty() {
        let parser = CliParser::new();
        let cmd = parser.parse(&[]).unwrap();
        assert_eq!(cmd, CliCommand::Help);
    }

    #[test]
    fn test_parse_help_flag() {
        let parser = CliParser::new();
        let cmd = parser.parse(&["--help".to_string()]).unwrap();
        assert_eq!(cmd, CliCommand::Help);
    }

    #[test]
    fn test_parse_start_default_model() {
        let parser = CliParser::new();
        let cmd = parser.parse(&["start".to_string()]).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Start {
                model: "qwen3.5:2b".to_string()
            }
        );
    }

    #[test]
    fn test_parse_start_custom_model() {
        let parser = CliParser::new();
        let cmd = parser
            .parse(&[
                "start".to_string(),
                "--model".to_string(),
                "qwen3.5:4b".to_string(),
            ])
            .unwrap();
        assert_eq!(
            cmd,
            CliCommand::Start {
                model: "qwen3.5:4b".to_string()
            }
        );
    }

    #[test]
    fn test_parse_audit() {
        let parser = CliParser::new();
        let cmd = parser
            .parse(&[
                "audit".to_string(),
                "--prompt".to_string(),
                "test".to_string(),
            ])
            .unwrap();
        assert_eq!(
            cmd,
            CliCommand::Audit {
                prompt: "test".to_string()
            }
        );
    }

    #[test]
    fn test_parse_audit_missing_prompt() {
        let parser = CliParser::new();
        let result = parser.parse(&["audit".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_status() {
        let parser = CliParser::new();
        let cmd = parser.parse(&["status".to_string()]).unwrap();
        assert_eq!(cmd, CliCommand::Status);
    }

    #[test]
    fn test_parse_credits() {
        let parser = CliParser::new();
        let cmd = parser.parse(&["credits".to_string()]).unwrap();
        assert_eq!(cmd, CliCommand::Credits);
    }

    #[test]
    fn test_parse_unknown_command() {
        let parser = CliParser::new();
        let result = parser.parse(&["unknown".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_start() {
        let runner = CliRunner::new();
        let output = runner.execute(&CliCommand::Start {
            model: "qwen3.5:2b".to_string(),
        });
        assert!(output.contains("qwen3.5:2b"));
    }

    #[test]
    fn test_execute_audit() {
        let runner = CliRunner::new();
        let output = runner.execute(&CliCommand::Audit {
            prompt: "hello".to_string(),
        });
        assert!(output.contains("hello"));
    }

    #[test]
    fn test_execute_status() {
        let runner = CliRunner::new();
        let output = runner.execute(&CliCommand::Status);
        assert!(output.contains("Node Status"));
    }

    #[test]
    fn test_execute_credits() {
        let runner = CliRunner::new();
        let output = runner.execute(&CliCommand::Credits);
        assert!(output.contains("Compute Credits"));
    }

    #[test]
    fn test_execute_help() {
        let runner = CliRunner::new();
        let output = runner.execute(&CliCommand::Help);
        assert!(output.contains("ed2kIA"));
        assert!(output.contains("start"));
        assert!(output.contains("audit"));
    }

    #[test]
    fn test_command_display() {
        let cmd = CliCommand::Start {
            model: "qwen3.5:2b".to_string(),
        };
        assert!(format!("{}", cmd).contains("start"));
    }

    #[test]
    fn test_run_help() {
        let result = run(&[]);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("ed2kIA"));
    }

    #[test]
    fn test_run_start() {
        let result = run(&[
            "start".to_string(),
            "--model".to_string(),
            "qwen3.5:4b".to_string(),
        ]);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("qwen3.5:4b"));
    }

    #[test]
    fn test_run_error() {
        let result = run(&["invalid".to_string()]);
        assert!(result.is_err());
    }
}
