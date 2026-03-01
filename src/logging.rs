//! Logging System
//!
//! Structured logging implementation using the `tracing` crate. Provides configurable
//! log levels, output formats, and destinations as specified in the logging specification.

use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing_subscriber::fmt::time::ChronoUtc;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

/// Resolve the log file path with precedence: CLI, MERKLE_LOG_FILE env, config file, default.
///
/// Default uses `ProjectDirs` state directory and optional workspace-scoped path segment.
pub fn resolve_log_file_path(
    cli_file: Option<PathBuf>,
    config_file: Option<PathBuf>,
    workspace: Option<&Path>,
) -> Result<PathBuf, ApiError> {
    if let Some(p) = cli_file {
        if !p.as_os_str().is_empty() {
            return Ok(p);
        }
    }
    if let Ok(env_path) = std::env::var("MERKLE_LOG_FILE") {
        if !env_path.is_empty() {
            return Ok(PathBuf::from(env_path));
        }
    }
    if let Some(p) = config_file {
        if !p.as_os_str().is_empty() {
            return Ok(p);
        }
    }
    default_log_file_path(workspace)
}

fn default_log_file_path(workspace: Option<&Path>) -> Result<PathBuf, ApiError> {
    let project_dirs = directories::ProjectDirs::from("", "meld", "meld").ok_or_else(|| {
        ApiError::ConfigError(
            "Could not determine platform state directory for log file".to_string(),
        )
    })?;
    let state_dir = project_dirs
        .state_dir()
        .ok_or_else(|| {
            ApiError::ConfigError(
                "Platform state directory not available for log file".to_string(),
            )
        })?
        .to_path_buf();
    let base = state_dir;
    let dir = match workspace {
        Some(ws) => {
            let canonical = ws.canonicalize().map_err(|e| {
                ApiError::ConfigError(format!("Failed to canonicalize workspace path: {}", e))
            })?;
            let mut path = base;
            for component in canonical.components() {
                match component {
                    std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
                    | std::path::Component::CurDir
                    | std::path::Component::ParentDir => {}
                    std::path::Component::Normal(name) => {
                        path = path.join(name);
                    }
                }
            }
            path
        }
        None => base,
    };
    Ok(dir.join("meld.log"))
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Whether logging is enabled (default: true)
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Log level: trace, debug, info, warn, error, off
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Output format: json, text (default: text)
    #[serde(default = "default_format")]
    pub format: String,

    /// Output destination: stdout, stderr, file, file+stderr, both
    #[serde(default = "default_output")]
    pub output: String,

    /// Log file path when output includes file; None means use runtime default
    #[serde(default)]
    pub file: Option<PathBuf>,

    /// Enable colored output (text format only, stdout/stderr only)
    #[serde(default = "default_true")]
    pub color: bool,

    /// Module-specific log levels
    #[serde(default)]
    pub modules: HashMap<String, String>,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_format() -> String {
    "text".to_string()
}

fn default_output() -> String {
    "file".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            level: default_log_level(),
            format: default_format(),
            output: default_output(),
            file: None,
            color: default_true(),
            modules: HashMap::new(),
        }
    }
}

/// Initialize the logging system
///
/// Priority order (highest to lowest):
/// 1. CLI arguments (passed via env vars or direct config)
/// 2. Environment variables (MERKLE_LOG, MERKLE_LOG_FORMAT, etc.)
/// 3. Configuration file
/// 4. Defaults
pub fn init_logging(config: Option<&LoggingConfig>) -> Result<(), ApiError> {
    let disabled = config
        .as_ref()
        .map(|c| !c.enabled)
        .unwrap_or(false);
    if disabled {
        Registry::default()
            .with(EnvFilter::new("off"))
            .with(fmt::layer().with_writer(|| std::io::sink()))
            .init();
        return Ok(());
    }

    let filter = build_env_filter(config)?;
    let format = determine_format(config)?;
    let output = determine_output(config)?;
    let use_color = config.map(|c| c.color).unwrap_or(true);

    let log_file_path = config
        .and_then(|c| c.file.clone())
        .or_else(|| resolve_log_file_path(None, None, None).ok());
    let get_file_writer = || -> Result<std::fs::File, ApiError> {
        let log_file = log_file_path.clone().ok_or_else(|| {
            ApiError::ConfigError("Log file path not set and default resolution failed".to_string())
        })?;
        if let Some(parent) = log_file.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ApiError::ConfigError(format!("Failed to create log directory: {}", e))
            })?;
        }
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .map_err(|e| {
                ApiError::ConfigError(format!("Failed to open log file {:?}: {}", log_file, e))
            })
    };

    let base_subscriber = Registry::default().with(filter);

    if format == "json" {
        if output.file && output.stderr {
            let file_writer = get_file_writer()?;
            let writer = file_writer.and(std::io::stderr);
            base_subscriber
                .with(
                    fmt::layer()
                        .json()
                        .with_target(true)
                        .with_timer(ChronoUtc::rfc_3339())
                        .with_writer(writer),
                )
                .init();
        } else if output.file {
            let file_writer = get_file_writer()?;
            base_subscriber
                .with(
                    fmt::layer()
                        .json()
                        .with_target(true)
                        .with_timer(ChronoUtc::rfc_3339())
                        .with_writer(file_writer),
                )
                .init();
        } else if output.stdout && output.stderr {
            let writer = std::io::stdout.and(std::io::stderr);
            base_subscriber
                .with(
                    fmt::layer()
                        .json()
                        .with_target(true)
                        .with_timer(ChronoUtc::rfc_3339())
                        .with_writer(writer),
                )
                .init();
        } else if output.stderr {
            base_subscriber
                .with(
                    fmt::layer()
                        .json()
                        .with_target(true)
                        .with_timer(ChronoUtc::rfc_3339())
                        .with_writer(std::io::stderr),
                )
                .init();
        } else {
            base_subscriber
                .with(
                    fmt::layer()
                        .json()
                        .with_target(true)
                        .with_timer(ChronoUtc::rfc_3339())
                        .with_writer(std::io::stdout),
                )
                .init();
        }
    } else {
        if output.file && output.stderr {
            let file_writer = get_file_writer()?;
            let writer = file_writer.and(std::io::stderr);
            base_subscriber
                .with(
                    fmt::layer()
                        .with_target(true)
                        .with_timer(ChronoUtc::rfc_3339())
                        .with_ansi(false)
                        .with_writer(writer),
                )
                .init();
        } else if output.file {
            let file_writer = get_file_writer()?;
            base_subscriber
                .with(
                    fmt::layer()
                        .with_target(true)
                        .with_timer(ChronoUtc::rfc_3339())
                        .with_ansi(false)
                        .with_writer(file_writer),
                )
                .init();
        } else if output.stdout && output.stderr {
            let writer = std::io::stdout.and(std::io::stderr);
            base_subscriber
                .with(
                    fmt::layer()
                        .with_target(true)
                        .with_timer(ChronoUtc::rfc_3339())
                        .with_ansi(use_color)
                        .with_writer(writer),
                )
                .init();
        } else if output.stderr {
            base_subscriber
                .with(
                    fmt::layer()
                        .with_target(true)
                        .with_timer(ChronoUtc::rfc_3339())
                        .with_ansi(use_color)
                        .with_writer(std::io::stderr),
                )
                .init();
        } else {
            base_subscriber
                .with(
                    fmt::layer()
                        .with_target(true)
                        .with_timer(ChronoUtc::rfc_3339())
                        .with_ansi(use_color)
                        .with_writer(std::io::stdout),
                )
                .init();
        }
    }

    Ok(())
}

/// Build environment filter from config or environment variables
fn build_env_filter(config: Option<&LoggingConfig>) -> Result<EnvFilter, ApiError> {
    // First, try to get filter from MERKLE_LOG environment variable
    let env_filter = EnvFilter::try_from_env("MERKLE_LOG");

    if let Ok(filter) = env_filter {
        return Ok(filter);
    }

    // Build filter from config
    let level = config.map(|c| c.level.as_str()).unwrap_or("info");

    if level == "off" {
        return Ok(EnvFilter::new("off"));
    }

    let mut filter = EnvFilter::new(level);

    // Add module-specific filters
    if let Some(config) = config {
        for (module, module_level) in &config.modules {
            let directive = format!("{}={}", module, module_level);
            filter = filter.add_directive(
                directive
                    .parse()
                    .map_err(|e| ApiError::ConfigError(format!("Invalid log directive: {}", e)))?,
            );
        }
    }

    // Also check MERKLE_LOG_MODULES environment variable
    if let Ok(modules_str) = std::env::var("MERKLE_LOG_MODULES") {
        for module_spec in modules_str.split(',') {
            let parts: Vec<&str> = module_spec.split('=').collect();
            if parts.len() == 2 {
                let directive = format!("{}={}", parts[0].trim(), parts[1].trim());
                filter = filter.add_directive(directive.parse().map_err(|e| {
                    ApiError::ConfigError(format!("Invalid log directive from env: {}", e))
                })?);
            }
        }
    }

    Ok(filter)
}

/// Determine output format from config or environment
fn determine_format(config: Option<&LoggingConfig>) -> Result<String, ApiError> {
    // Check environment variable first
    if let Ok(format) = std::env::var("MERKLE_LOG_FORMAT") {
        if format == "json" || format == "text" {
            return Ok(format);
        }
    }

    // Use config
    let format = config.map(|c| c.format.as_str()).unwrap_or("text");

    if format != "json" && format != "text" {
        return Err(ApiError::ConfigError(format!(
            "Invalid log format: {} (must be 'json' or 'text')",
            format
        )));
    }

    Ok(format.to_string())
}

/// Output destinations
struct OutputDestinations {
    stdout: bool,
    stderr: bool,
    file: bool,
}

/// Determine output destinations from config or environment
fn determine_output(config: Option<&LoggingConfig>) -> Result<OutputDestinations, ApiError> {
    if let Ok(output) = std::env::var("MERKLE_LOG_OUTPUT") {
        return parse_output_destinations(&output);
    }
    let output = config.map(|c| c.output.as_str()).unwrap_or("file");
    parse_output_destinations(output)
}

fn parse_output_destinations(output: &str) -> Result<OutputDestinations, ApiError> {
    match output {
        "stdout" => Ok(OutputDestinations {
            stdout: true,
            stderr: false,
            file: false,
        }),
        "stderr" => Ok(OutputDestinations {
            stdout: false,
            stderr: true,
            file: false,
        }),
        "file" => Ok(OutputDestinations {
            stdout: false,
            stderr: false,
            file: true,
        }),
        "file+stderr" => Ok(OutputDestinations {
            stdout: false,
            stderr: true,
            file: true,
        }),
        "both" => Ok(OutputDestinations {
            stdout: true,
            stderr: true,
            file: false,
        }),
        _ => Err(ApiError::ConfigError(format!(
            "Invalid log output: {} (must be 'stdout', 'stderr', 'file', 'file+stderr', or 'both')",
            output
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_logging_config() {
        let config = LoggingConfig::default();
        assert!(config.enabled);
        assert_eq!(config.level, "info");
        assert_eq!(config.format, "text");
        assert_eq!(config.output, "file");
        assert_eq!(config.file, None);
        assert!(config.color);
    }

    #[test]
    fn test_parse_output_destinations() {
        let out = parse_output_destinations("stdout").unwrap();
        assert!(out.stdout);
        assert!(!out.stderr);
        assert!(!out.file);

        let out = parse_output_destinations("both").unwrap();
        assert!(out.stdout);
        assert!(out.stderr);
        assert!(!out.file);

        let out = parse_output_destinations("file+stderr").unwrap();
        assert!(!out.stdout);
        assert!(out.stderr);
        assert!(out.file);
    }

    #[test]
    fn test_resolve_log_file_path_cli_wins() {
        let cli = Some(PathBuf::from("/tmp/cli.log"));
        let config = Some(PathBuf::from("/tmp/config.log"));
        let path = resolve_log_file_path(cli, config, None).unwrap();
        assert_eq!(path, PathBuf::from("/tmp/cli.log"));
    }

    #[test]
    fn test_resolve_log_file_path_config_when_cli_none() {
        let config = Some(PathBuf::from("/tmp/config.log"));
        let path = resolve_log_file_path(None, config, None).unwrap();
        assert_eq!(path, PathBuf::from("/tmp/config.log"));
    }

    #[test]
    fn test_resolve_log_file_path_default_fallback() {
        let path = resolve_log_file_path(None, None, None).unwrap();
        assert!(path.ends_with("meld.log"));
        assert!(path.components().count() >= 2);
    }

    #[test]
    fn test_resolve_log_file_path_default_with_workspace() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path();
        let path = resolve_log_file_path(None, None, Some(workspace)).unwrap();
        assert!(path.ends_with("meld.log"));
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains("meld"),
            "path should contain meld segment: {}",
            path_str
        );
    }

    #[test]
    fn test_resolve_log_file_path_env_wins_over_config() {
        let config = Some(PathBuf::from("/tmp/config.log"));
        std::env::set_var("MERKLE_LOG_FILE", "/env/meld.log");
        let result = resolve_log_file_path(None, config, None);
        std::env::remove_var("MERKLE_LOG_FILE");
        let path = result.unwrap();
        assert_eq!(path, PathBuf::from("/env/meld.log"));
    }
}
