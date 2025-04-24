//! # DevRS Configuration System
//!
//! File: cli/src/core/config.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten 
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the configuration system for DevRS, handling loading,
//! merging, validation, and access to configuration data. It supports a multi-level
//! configuration approach that combines defaults, user settings, and project-specific
//! overrides.
//!
//! ## Architecture
//!
//! The configuration system follows these principles:
//! - Configuration is loaded from multiple sources in order of precedence
//! - Paths are validated and expanded (e.g., `~` to home directory)
//! - Configuration is validated for correctness before use
//! - Structured data models ensure type safety
//!
//! Configuration sources (in order of precedence):
//! 1. Project-specific `.devrs.toml` in current directory or ancestors
//! 2. User-specific `~/.config/devrs/config.toml`
//! 3. Default values defined in the code
//!
//! ## Examples
//!
//! Loading and using configuration:
//!
//! ```rust
//! let cfg = config::load_config()?;
//!
//! // Access core environment settings
//! let image_name = &cfg.core_env.image_name;
//! let mounts = &cfg.core_env.mounts;
//!
//! // Access blueprint directory
//! let blueprint_dir = &cfg.blueprints.directory;
//! ```
//!
//! The configuration is loaded once per command execution and passed
//! to the modules that need it.
//!
use crate::core::error::{DevrsError, Result}; // Use error from the same core module
use anyhow::{anyhow, Context};
use directories::ProjectDirs;
use serde::Deserialize;
use std::collections::HashMap;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tracing::{debug, info, warn};

// --- Struct definitions (Config, CoreEnvConfig, MountConfig, BlueprintsConfig, ApplicationDefaults) remain the same ---
// ... (Paste the struct definitions from the previous response here) ...
/// Represents the main configuration structure, loaded from TOML files.
#[derive(Deserialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)] // Error if unknown fields are in TOML
pub struct Config {
    #[serde(default)]
    pub core_env: CoreEnvConfig,
    #[serde(default)]
    pub blueprints: BlueprintsConfig,
    #[serde(default)]
    pub application_defaults: ApplicationDefaults,
    // Add other top-level configuration sections here
}

/// Configuration specific to the core development environment (`devrs env ...`).
#[derive(Deserialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct CoreEnvConfig {
    /// List of volume mounts.
    #[serde(default)]
    pub mounts: Vec<MountConfig>,
    /// List of port mappings (e.g., "8080:80").
    #[serde(default)]
    pub ports: Vec<String>,
    /// Environment variables to set inside the container.
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    /// Default working directory inside the container.
    #[serde(default = "default_core_workdir")]
    pub default_workdir: String,
    /// Name of the Docker image to use for the core environment.
    #[serde(default = "default_core_image")]
    pub image_name: String,
    /// Tag of the Docker image to use.
    #[serde(default = "default_core_image_tag")]
    pub image_tag: String,
}

/// Configuration for a single volume mount.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MountConfig {
    /// Path on the host machine (can use ~). Will be expanded.
    pub host: String,
    /// Path inside the container.
    pub container: String,
    /// Mount as read-only (defaults to false).
    #[serde(default)]
    pub readonly: bool,
}

/// Configuration related to project blueprints (`devrs blueprint ...`).
#[derive(Deserialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct BlueprintsConfig {
    /// Directory where blueprint templates are stored (can use ~). Will be expanded.
    #[serde(default = "default_blueprint_dir")]
    pub directory: String,
}

/// Optional default settings for application containers (`devrs container ...`).
#[derive(Deserialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct ApplicationDefaults {
    /// Default prefix for application image tags.
    pub default_image_prefix: Option<String>,
    /// Default ports to map for `devrs container run`.
    #[serde(default)]
    pub default_ports: Vec<String>,
}

// --- Default value functions (default_core_workdir, default_blueprint_dir, etc.) remain the same ---
// ... (Paste the default value functions from the previous response here) ...
fn default_core_workdir() -> String {
    "/home/me/code".to_string()
}
fn default_blueprint_dir() -> String {
    "~/.config/devrs/blueprints".to_string() // Sensible default user location
}
fn default_core_image() -> String {
    "devrs-core-env".to_string()
}
fn default_core_image_tag() -> String {
    "latest".to_string()
}

// --- Configuration Loading Functions (load_config, load_user_config, etc.) remain the same ---
// ... (Paste the loading functions from the previous response here) ...
const PROJECT_CONFIG_FILENAME: &str = ".devrs.toml";

pub fn load_config() -> Result<Config> {
    let user_config = load_user_config()?;
    let project_config = load_project_config()?;
    let mut merged_config = merge_configs(user_config.unwrap_or_default(), project_config);
    expand_config_paths(&mut merged_config).context("Failed to expand paths in configuration")?;
    validate_config(&merged_config).context("Configuration validation failed")?;
    debug!("Final loaded configuration: {:?}", merged_config);
    Ok(merged_config)
}

fn load_user_config() -> Result<Option<Config>> {
    if let Some(proj_dirs) = ProjectDirs::from("com", "DevRS", "devrs") {
        let config_dir = proj_dirs.config_dir();
        let config_path = config_dir.join("config.toml");
        if config_path.exists() {
            info!("Loading user configuration from: {}", config_path.display());
            load_config_from_path(&config_path).map(Some)
        } else {
            debug!(
                "User configuration file not found at {}",
                config_path.display()
            );
            Ok(None)
        }
    } else {
        warn!("Could not determine user config directory.");
        Ok(None)
    }
}

fn load_project_config() -> Result<Option<Config>> {
    if let Some(project_config_path) = find_project_config_path()? {
        info!(
            "Loading project configuration from: {}",
            project_config_path.display()
        );
        load_config_from_path(&project_config_path).map(Some)
    } else {
        debug!(
            "No project configuration file (.devrs.toml) found in current directory or ancestors."
        );
        Ok(None)
    }
}

fn find_project_config_path() -> Result<Option<PathBuf>> {
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    let mut path: &Path = &current_dir;
    loop {
        let project_config = path.join(PROJECT_CONFIG_FILENAME);
        let git_dir = path.join(".git");
        if project_config.exists() && project_config.is_file() {
            return Ok(Some(project_config));
        }
        if git_dir.exists() && git_dir.is_dir() {
            debug!(
                "Found .git directory at {}, stopping project config search.",
                path.display()
            );
            return Ok(None);
        }
        match path.parent() {
            Some(parent) => path = parent,
            None => break,
        }
    }
    Ok(None)
}

fn load_config_from_path(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read configuration file: {}", path.display()))?;
    toml::from_str(&content)
        .with_context(|| format!("Failed to parse TOML from file: {}", path.display()))
}

fn merge_configs(user: Config, project: Option<Config>) -> Config {
    let project_cfg = match project {
        Some(p) => p,
        None => return user,
    };
    let mut merged = Config::default();
    merged.core_env.image_name = if project_cfg.core_env.image_name != default_core_image() {
        project_cfg.core_env.image_name
    } else {
        user.core_env.image_name
    };
    merged.core_env.image_tag = if project_cfg.core_env.image_tag != default_core_image_tag() {
        project_cfg.core_env.image_tag
    } else {
        user.core_env.image_tag
    };
    merged.core_env.default_workdir =
        if project_cfg.core_env.default_workdir != default_core_workdir() {
            project_cfg.core_env.default_workdir
        } else {
            user.core_env.default_workdir
        };
    merged.core_env.mounts = if !project_cfg.core_env.mounts.is_empty() {
        project_cfg.core_env.mounts
    } else {
        user.core_env.mounts
    };
    merged.core_env.ports = if !project_cfg.core_env.ports.is_empty() {
        project_cfg.core_env.ports
    } else {
        user.core_env.ports
    };
    merged.core_env.env_vars = if !project_cfg.core_env.env_vars.is_empty() {
        project_cfg.core_env.env_vars
    } else {
        user.core_env.env_vars
    };
    merged.blueprints.directory = if project_cfg.blueprints.directory != default_blueprint_dir() {
        project_cfg.blueprints.directory
    } else {
        user.blueprints.directory
    };
    merged.application_defaults.default_image_prefix = project_cfg
        .application_defaults
        .default_image_prefix
        .or(user.application_defaults.default_image_prefix);
    merged.application_defaults.default_ports =
        if !project_cfg.application_defaults.default_ports.is_empty() {
            project_cfg.application_defaults.default_ports
        } else {
            user.application_defaults.default_ports
        };
    merged
}

fn expand_config_paths(config: &mut Config) -> Result<()> {
    debug!("Expanding paths in configuration...");
    config.blueprints.directory = shellexpand::tilde(&config.blueprints.directory).into_owned();
    debug!(
        "Expanded blueprint directory: {}",
        config.blueprints.directory
    );
    for mount in &mut config.core_env.mounts {
        mount.host = shellexpand::tilde(&mount.host).into_owned();
        debug!("Expanded mount host path: {}", mount.host);
    }
    Ok(())
}

fn validate_config(config: &Config) -> Result<()> {
    info!("Validating final configuration...");
    let bp_dir = PathBuf::from(&config.blueprints.directory);
    if !bp_dir.exists() {
        warn!(
            "Configured blueprint directory '{}' does not exist.",
            bp_dir.display()
        );
    } else if !bp_dir.is_dir() {
        return Err(anyhow!(DevrsError::Config(format!(
            "Configured blueprint path '{}' exists but is not a directory.",
            bp_dir.display()
        ))));
    }
    for port_mapping in &config.core_env.ports {
        if !port_mapping.contains(':') || port_mapping.matches(':').count() != 1 {
            return Err(anyhow!(DevrsError::Config(format!(
                "Invalid port mapping format: '{}'. Expected HOST:CONTAINER.",
                port_mapping
            ))));
        }
    }
    for mount in &config.core_env.mounts {
        if mount.host.is_empty() {
            return Err(anyhow!(DevrsError::Config(format!(
                "Mount configuration cannot have an empty host path (container path: '{}').",
                mount.container
            ))));
        }
        if mount.container.is_empty() {
            return Err(anyhow!(DevrsError::Config(format!(
                "Mount configuration cannot have an empty container path (host path: '{}').",
                mount.host
            ))));
        }
    }
    info!("Configuration validation successful.");
    Ok(())
}

// --- Unit Tests (Config tests) remain the same ---
// ... (Paste the tests from the previous response here) ...
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_deserialize_basic_toml() {
        let toml_content = r#"
            [core_env]
            default_workdir = "/workspace"
            image_name = "my-custom-env"
            ports = ["9000:8080"]

            [[core_env.mounts]]
            host = "~/my_projects"
            container = "/projects"

            [blueprints]
            directory = "/etc/devrs/blueprints"
        "#;

        let config: Config = toml::from_str(toml_content).expect("Failed to parse TOML");

        assert_eq!(config.core_env.default_workdir, "/workspace");
        assert_eq!(config.core_env.image_name, "my-custom-env");
        assert_eq!(config.core_env.image_tag, default_core_image_tag()); // Default
        assert_eq!(config.core_env.ports, vec!["9000:8080"]);
        assert_eq!(config.core_env.mounts.len(), 1);
        assert_eq!(config.core_env.mounts[0].host, "~/my_projects"); // Not yet expanded
        assert_eq!(config.core_env.mounts[0].container, "/projects");
        assert!(!config.core_env.mounts[0].readonly); // Default readonly
        assert_eq!(config.blueprints.directory, "/etc/devrs/blueprints"); // Not yet expanded
    }

    #[test]
    fn test_path_expansion() {
        let mut config = Config {
            blueprints: BlueprintsConfig {
                directory: "~/bp_test".to_string(),
            },
            core_env: CoreEnvConfig {
                mounts: vec![
                    MountConfig {
                        host: "~/code".to_string(),
                        container: "/code".to_string(),
                        readonly: false,
                    },
                    MountConfig {
                        host: "/absolute/path".to_string(),
                        container: "/abs".to_string(),
                        readonly: true,
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        };

        expand_config_paths(&mut config).unwrap();

        let home_dir = dirs::home_dir().unwrap();
        assert_eq!(
            config.blueprints.directory,
            home_dir.join("bp_test").to_string_lossy()
        );
        assert_eq!(
            config.core_env.mounts[0].host,
            home_dir.join("code").to_string_lossy()
        );
        assert_eq!(config.core_env.mounts[1].host, "/absolute/path"); // Absolute path unchanged
    }

    #[test]
    #[ignore] // Integration tests require complex mocking or real fs/env setup
    fn test_load_config_integration_no_files() {}

    #[test]
    #[ignore] // Integration tests require complex mocking or real fs/env setup
    fn test_load_config_integration_with_files() {}

    #[test]
    fn test_validate_config_valid() {
        let temp_dir = tempdir().unwrap();
        fs::create_dir(temp_dir.path().join("bps")).unwrap(); // Create blueprint dir

        let config = Config {
            blueprints: BlueprintsConfig {
                directory: temp_dir.path().join("bps").to_string_lossy().to_string(),
            },
            core_env: CoreEnvConfig {
                ports: vec!["8080:80".to_string()],
                mounts: vec![MountConfig {
                    host: "/host/path".into(),
                    container: "/container/path".into(),
                    readonly: false,
                }],
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_invalid_port() {
        let config = Config {
            core_env: CoreEnvConfig {
                ports: vec!["invalid".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };
        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid port mapping format"));
    }

    #[test]
    fn test_validate_config_blueprint_path_is_file() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("not_a_dir");
        fs::write(&file_path, "").unwrap();

        let config = Config {
            blueprints: BlueprintsConfig {
                directory: file_path.to_string_lossy().to_string(),
            },
            ..Default::default()
        };
        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("is not a directory"));
    }
}
