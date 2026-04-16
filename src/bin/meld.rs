//! Meld CLI Binary
//!
//! Command-line interface for the Meld filesystem state management system.

use clap::Parser;
use meld::cli::{Cli, Commands, DangerCommands, RunContext};
use meld::config::ConfigLoader;
use meld::logging::{LoggingConfig, init_logging};
use std::path::{Path, PathBuf};
use std::process;
use tracing::{error, info};

fn main() {
    let cli = Cli::parse();
    let logging_workspace =
        danger_workspace_override(&cli).unwrap_or_else(|| cli.workspace.clone());

    // Build logging config from CLI args, env vars, and config file
    let logging_config = build_logging_config(&cli, logging_workspace.as_path());

    // Initialize logging early
    if let Err(e) = init_logging(Some(&logging_config)) {
        eprintln!("Failed to initialize logging: {}", e);
        process::exit(1);
    }

    info!("Meld CLI starting");

    if let Some(result) = try_execute_danger_command(&cli) {
        match result {
            Ok(output) => {
                info!("Danger command completed successfully");
                println!("{}", output);
            }
            Err(e) => {
                error!("Command failed: {}", e);
                eprintln!("{}", meld::cli::map_error(&e));
                process::exit(1);
            }
        }
        return;
    }

    if let Some(result) = try_execute_branch_command(&cli) {
        match result {
            Ok(output) => {
                info!("Branch command completed successfully");
                println!("{}", output);
            }
            Err(e) => {
                error!("Command failed: {}", e);
                eprintln!("{}", meld::cli::map_error(&e));
                process::exit(1);
            }
        }
        return;
    }

    // Create CLI context
    let context = match RunContext::new(cli.workspace.clone(), cli.config.clone()) {
        Ok(ctx) => {
            info!("CLI context initialized");
            ctx
        }
        Err(e) => {
            error!("Error initializing workspace: {}", e);
            eprintln!("{}", meld::cli::map_error(&e));
            process::exit(1);
        }
    };

    // Execute command
    match context.execute(&cli.command) {
        Ok(output) => {
            info!("Command completed successfully");
            println!("{}", output);
        }
        Err(e) => {
            error!("Command failed: {}", e);
            eprintln!("{}", meld::cli::map_error(&e));
            process::exit(1);
        }
    }
}

fn try_execute_danger_command(cli: &Cli) -> Option<Result<String, meld::error::ApiError>> {
    match &cli.command {
        Commands::Danger {
            command:
                DangerCommands::Flush {
                    path,
                    path_positional,
                    dry_run,
                    yes,
                },
        } => {
            let workspace_root = path
                .clone()
                .or_else(|| path_positional.clone())
                .unwrap_or_else(|| cli.workspace.clone());
            Some(meld::workspace::WorkspaceDangerService::flush(
                &workspace_root,
                cli.config.as_deref(),
                *dry_run,
                *yes,
            ))
        }
        _ => None,
    }
}

fn try_execute_branch_command(cli: &Cli) -> Option<Result<String, meld::error::ApiError>> {
    match &cli.command {
        Commands::Branches { command } => {
            Some(meld::branches::tooling::handle_cli_command_with_workspace(
                command,
                Some(cli.workspace.as_path()),
            ))
        }
        _ => None,
    }
}

fn danger_workspace_override(cli: &Cli) -> Option<PathBuf> {
    match &cli.command {
        Commands::Danger {
            command:
                DangerCommands::Flush {
                    path,
                    path_positional,
                    ..
                },
        } => path.clone().or_else(|| path_positional.clone()),
        _ => None,
    }
}

/// Build logging configuration from CLI args, environment, and config file.
/// Precedence: CLI flags override config file override defaults.
fn build_logging_config(cli: &Cli, logging_workspace: &Path) -> LoggingConfig {
    let mut config = if let Some(ref config_path) = cli.config {
        ConfigLoader::load_from_file(config_path)
            .ok()
            .map(|c| c.logging)
            .unwrap_or_default()
    } else {
        ConfigLoader::load(logging_workspace)
            .ok()
            .map(|c| c.logging)
            .unwrap_or_default()
    };

    if cli.quiet {
        config.enabled = false;
    }
    if cli.verbose {
        config.level = "debug".to_string();
        // Make verbose mode observable in terminal output without losing file logs.
        // An explicit --log-output value still takes precedence below.
        if config.output == "file" {
            config.output = "file+stderr".to_string();
        }
    }
    if let Some(ref level) = cli.log_level {
        config.level = level.clone();
    }
    if let Some(ref format) = cli.log_format {
        config.format = format.clone();
    }
    if let Some(ref output) = cli.log_output {
        config.output = output.clone();
    }

    let output_uses_file = config.output == "file" || config.output == "file+stderr";
    if config.enabled && output_uses_file {
        let resolved = meld::logging::resolve_log_file_path(
            cli.log_file.clone(),
            config.file.clone(),
            Some(logging_workspace),
        );
        if let Ok(path) = resolved {
            config.file = Some(path);
        }
    } else if let Some(ref file) = cli.log_file {
        config.file = Some(file.clone());
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use meld::cli::Cli;

    #[test]
    fn test_build_logging_config_default() {
        let temp = tempfile::tempdir().unwrap();
        let ws = temp.path().to_string_lossy();
        let cli = Cli::try_parse_from(["meld", "--workspace", ws.as_ref(), "status"]).unwrap();
        let config = build_logging_config(&cli, temp.path());
        assert!(config.enabled, "default should have logging enabled");
        assert_eq!(config.output, "file", "default output should be file");
        assert_eq!(config.level, "info", "default level should be info");
    }

    #[test]
    fn test_build_logging_config_quiet() {
        let cli = Cli::try_parse_from(["meld", "--quiet", "status"]).unwrap();
        let cwd = std::env::current_dir().unwrap();
        let config = build_logging_config(&cli, cwd.as_path());
        assert!(!config.enabled, "quiet should disable logging");
    }

    #[test]
    fn test_build_logging_config_verbose() {
        let temp = tempfile::tempdir().unwrap();
        let ws = temp.path().to_string_lossy();
        let cli = Cli::try_parse_from(["meld", "--workspace", ws.as_ref(), "--verbose", "status"])
            .unwrap();
        let config = build_logging_config(&cli, temp.path());
        assert_eq!(config.level, "debug", "verbose should set level to debug");
        assert_eq!(
            config.output, "file+stderr",
            "verbose should mirror logs to stderr when default output is file"
        );
    }

    #[test]
    fn test_build_logging_config_verbose_respects_explicit_output_override() {
        let temp = tempfile::tempdir().unwrap();
        let ws = temp.path().to_string_lossy();
        let cli = Cli::try_parse_from([
            "meld",
            "--workspace",
            ws.as_ref(),
            "--verbose",
            "--log-output",
            "stderr",
            "status",
        ])
        .unwrap();
        let config = build_logging_config(&cli, temp.path());
        assert_eq!(config.level, "debug");
        assert_eq!(
            config.output, "stderr",
            "explicit --log-output should win over verbose defaults"
        );
    }
}
