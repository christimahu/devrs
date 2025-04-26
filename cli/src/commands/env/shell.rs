//! # DevRS Environment Shell Handler
//!
//! File: cli/src/commands/env/shell.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs env shell` command. This is the **primary command**
//! used by developers to interact with the **core DevRS development environment**.
//! It ensures the core environment container exists and is running, then attaches
//! an interactive shell (typically bash) to it, allowing the user to work directly
//! within the pre-configured development environment.
//!
//! ## Architecture
//!
//! The command execution follows these steps:
//! 1. Parse command-line arguments (`ShellArgs`) using `clap`, primarily the optional `--name` override.
//! 2. Attempt to load the DevRS configuration (`core::config`). If loading fails or the loaded config
//!    has empty image details, it gracefully falls back to using a minimal default configuration
//!    created in memory by `create_minimal_default_config`.
//! 3. Determine the target core environment container name using `get_core_env_container_name`
//!    (honoring the `--name` override or using the name derived from the loaded/default config).
//! 4. Call the shared `common::docker::lifecycle::ensure_core_env_running` utility function.
//!    This function checks if the target container exists and is running. If not, it creates
//!    and/or starts the container using the image name, mounts, ports, etc., from the loaded/default config.
//!    It returns a boolean indicating if the container was newly created.
//! 5. Determine the shell command to run inside the container (defaults to `/bin/bash`).
//! 6. Retrieve the working directory path from the loaded/default configuration (`core_env.default_workdir`).
//! 7. Call the shared `common::docker::interaction::exec_in_container` utility function to start an
//!    interactive (`-it`) shell session inside the container, using the determined shell command and working directory.
//! 8. Handle the exit code returned by the shell session.
//!
//! ## Usage
//!
//! ```bash
//! # Start an interactive shell in the default core development environment
//! devrs env shell
//!
//! # Start a shell in a specifically named core environment container
//! devrs env shell --name my-custom-env-instance
//! ```
//!
//! This command ensures a seamless entry into the consistent development environment provided by DevRS.
//!
use crate::{
    common::docker::{self}, // Access shared Docker utilities (ensure_running, exec).
    core::{
        config, // Access configuration loading and structures.
        error::Result, // Standard Result type for error handling.
    },
};
use anyhow::Context; // For adding context to errors.
use clap::Parser; // For parsing command-line arguments.
use std::env; // Required for getting the current directory (for default config).
use tracing::{debug, info, warn}; // Logging framework utilities.

/// # Environment Shell Arguments (`ShellArgs`)
///
/// Defines the command-line arguments accepted by the `devrs env shell` subcommand.
/// Uses the `clap` crate for parsing and validation.
#[derive(Parser, Debug)]
#[command(
    about = "Start an interactive shell in the core development environment",
    long_about = "Ensures the core environment container exists and is running, then opens an interactive shell.\n\
                  If the container doesn't exist, it will be created from the configured image.\n\
                  If it's stopped, it will be started.\n\
                  If no configuration file is found or it lacks image details, defaults will be used\n\
                  (image 'devrs-env:latest', mounting current directory to /code, workdir /code)."
)]
pub struct ShellArgs {
    /// Optional: Specifies the exact name of the core environment container to connect to.
    /// If omitted, the default name (derived from the `core_env.image_name` in the configuration,
    /// typically `<image_name>-instance`) is used. This allows managing multiple named core
    /// environment instances if needed, although typically only one is used.
    #[arg(long)] // Define as `--name <NAME>`.
    name: Option<String>,
    // TODO: Consider adding `--user` or `--command` overrides later if specific use cases arise,
    // similar to `devrs env exec`. Currently, it defaults to `/bin/bash` as the container's default user.
}

/// # Handle Environment Shell Command (`handle_shell`)
///
/// The main asynchronous handler function for the `devrs env shell` command.
/// This is the primary entry point for users to work within the core development environment.
/// It ensures the environment container is running and attaches an interactive shell.
///
/// ## Workflow:
/// 1.  Logs the start and parsed arguments.
/// 2.  Attempts to load the DevRS configuration using `core::config::load_config`.
/// 3.  **Fallback:** If loading fails (e.g., file not found, parse error) or if the loaded config
///     lacks essential image details (empty `image_name` or `image_tag`), it calls
///     `create_minimal_default_config` to generate a basic, usable configuration in memory.
///     A warning is logged, and a message is printed informing the user about the defaults being used.
/// 4.  Determines the target container name using `get_core_env_container_name`, honoring the `--name`
///     argument or using the name derived from the (loaded or default) configuration.
/// 5.  Calls `common::docker::lifecycle::ensure_core_env_running`, passing the container name and the
///     active configuration (loaded or default). This crucial step handles container creation
///     (using config mounts, ports, image) and starting as needed.
/// 6.  Prints informational messages if the container was newly created, especially if default settings were used.
/// 7.  Defines the shell command to run inside the container (currently hardcoded to `/bin/bash`).
/// 8.  Retrieves the target working directory from the active configuration (`core_env.default_workdir`).
/// 9.  Calls `common::docker::interaction::exec_in_container` to start the interactive (`-it`) shell session
///     using the determined shell command and working directory.
/// 10. Logs the exit code of the shell session upon completion.
///
/// ## Arguments
///
/// * `args`: The parsed `ShellArgs` struct containing the optional container `name`.
///
/// ## Returns
///
/// * `Result<()>`: Returns `Ok(())` if the shell session starts and completes without Docker errors.
///   Note: It currently returns `Ok(())` even if the shell itself exits with a non-zero code.
/// * `Err`: Returns an `Err` if config loading fails critically (rare), if the container cannot be
///   prepared (e.g., image pull fails, Docker daemon error), or if the Docker `exec` operation fails.
pub async fn handle_shell(args: ShellArgs) -> Result<()> {
    info!("Handling env shell command..."); // Log entry point.
    debug!("Shell args: {:?}", args); // Log parsed arguments.

    // 1. Attempt to load configuration, handling failure by using defaults.
    let cfg_result = config::load_config(); //

    // Variable to hold the active configuration (either loaded or default).
    let cfg: config::Config;
    // Flag to track if we fell back to using default settings.
    let mut using_defaults = false;

    // Process the result of config loading.
    match cfg_result {
        Ok(loaded_cfg) => {
            // Config loaded, but check if essential core_env fields are present.
            if loaded_cfg.core_env.image_name.trim().is_empty()
                || loaded_cfg.core_env.image_tag.trim().is_empty()
            {
                // Log and inform user if required fields are missing, then use defaults.
                warn!(
                    "Loaded configuration has empty image name or tag. Falling back to defaults."
                );
                println!("Warning: Loaded configuration missing core image details. Using defaults.");
                cfg = create_minimal_default_config().await?; // Generate default config.
                using_defaults = true; // Mark that defaults are being used.
            } else {
                // Loaded config is valid.
                info!("Successfully loaded configuration.");
                cfg = loaded_cfg; // Use the loaded configuration.
            }
        }
        Err(e) => {
            // Config loading itself failed (e.g., parse error, critical IO error).
            // Log warning, inform user, and use defaults.
            warn!(
                "Could not load configuration ({}). Proceeding with minimal defaults.",
                e
            );
            println!("Note: Configuration file not found or invalid. Using default settings (image: devrs-env:latest, mounting current directory to /code, workdir /code).");
            cfg = create_minimal_default_config().await?; // Generate default config.
            using_defaults = true; // Mark that defaults are being used.
        }
    }

    // 2. Determine the target container name.
    let container_name = args.name.clone().unwrap_or_else(|| {
        // If --name not specified, derive default name from the active config (loaded or default).
        let default_name = get_core_env_container_name(&cfg);
        debug!("Using container name: {}", default_name);
        default_name
    });

    // 3. Ensure the container exists and is running. This handles creation/starting automatically.
    // Pass the active config (`cfg`) which contains image name, mounts, ports etc. needed for creation.
    let needs_creation = docker::lifecycle::ensure_core_env_running(&container_name, &cfg) //
        .await // Await the async operation.
        .with_context(|| format!("Failed to prepare container '{}' for shell", container_name))?;
    // `needs_creation` indicates if the container was created in this call.

    // If the container was just created *and* we used default settings, provide extra info.
    if needs_creation {
        info!("Container '{}' was newly created.", container_name);
        if using_defaults {
            println!(
                "Created container '{}' using default image '{}:{}'.", // Combine format string
                container_name,
                cfg.core_env.image_name, // Pass args directly
                cfg.core_env.image_tag   // Pass args directly
            );
            println!("Current host directory mounted to /code inside the container.");
            println!("Default working directory set to /code.");
        }
    }

    // 4. Execute the interactive shell inside the prepared container.
    println!(
        "Starting interactive shell in core environment container '{}'...",
        container_name
    );
    println!("(Type 'exit' or press Ctrl+D to leave the shell)");

    // Determine the shell command (currently hardcoded, could be configurable later).
    let shell_cmd = vec!["/bin/bash".to_string()]; // Default to bash.
    info!("Executing shell command: {:?}", shell_cmd);

    // Get the working directory specified in the active config.
    let workdir_to_use = cfg.core_env.default_workdir.clone();
    info!("Using working directory: {}", workdir_to_use);

    // Use the shared `exec_in_container` utility to run the shell interactively.
    let exit_code = docker::interaction::exec_in_container( //
        &container_name,        // Target container name.
        &shell_cmd,             // Command to run (the shell).
        true,                   // interactive = true: Attach stdin.
        true,                   // tty = true: Allocate a pseudo-terminal.
        Some(&workdir_to_use),  // Set the working directory inside the container.
        None,                   // user = None: Run as container's default user.
    )
    .await // Await the async execution.
    .with_context(|| { // Add context if the exec operation fails.
        format!(
            "Failed to execute interactive shell in container '{}'",
            container_name
        )
    })?;

    // Log the exit code of the shell session.
    if exit_code == 0 {
        info!(
            "Shell exited successfully from container '{}'.",
            container_name
        );
    } else {
        // Note: Currently, a non-zero exit from the shell does *not* cause `handle_shell`
        // to return an Err. This might be desired behavior (the command itself ran, even if the shell exited abnormally).
        warn!(
            "Shell exited with code {} from container '{}'.",
            exit_code, container_name
        );
    }

    Ok(()) // Return Ok, indicating the `devrs env shell` command itself completed.
}

/// # Get Core Environment Container Name (`get_core_env_container_name`)
///
/// Helper function to consistently derive the default name for the core development
/// environment container based on the image name defined in the configuration.
///
/// ## Logic:
/// Appends "-instance" to the configured `core_env.image_name`.
///
/// ## Arguments
///
/// * `cfg`: A reference (`&`) to the loaded `Config` struct.
///
/// ## Returns
///
/// * `String`: The derived default container name (e.g., "devrs-core-tools-instance").
fn get_core_env_container_name(cfg: &config::Config) -> String {
    // Use format! for simple string construction.
    format!("{}-instance", cfg.core_env.image_name)
}

/// # Create Minimal Default Config (`create_minimal_default_config`)
///
/// Creates a basic, usable `config::Config` object in memory. This is used as a fallback
/// when the actual configuration file (`~/.config/devrs/config.toml` or `.devrs.toml`)
/// is missing, cannot be parsed, or lacks essential core environment details (image name/tag).
///
/// ## Default Settings Generated:
/// - **Image:** "devrs-env:latest"
/// - **Mounts:** Mounts the current host working directory to `/code` inside the container.
/// - **Working Directory:** Sets the container's working directory to `/code`.
/// - **Ports/Env Vars:** None by default.
/// - **Blueprints/App Defaults:** Empty/Default.
///
/// This ensures that `devrs env shell` can still function minimally even without explicit user configuration.
///
/// ## Returns
///
/// * `Result<config::Config>`: Returns the generated default `Config` object.
/// * `Err`: Returns an `Err` if the current working directory cannot be determined (rare).
async fn create_minimal_default_config() -> Result<config::Config> {
    // Get the current working directory of the host where `devrs` was invoked.
    let current_dir = env::current_dir().context("Failed to get current directory")?;
    // Convert the PathBuf to a String for storing in the config struct.
    let host_path = current_dir.to_string_lossy().to_string();

    // Construct the Config object with default values.
    Ok(config::Config {
        core_env: config::CoreEnvConfig {
            // Use hardcoded default image name and tag.
            image_name: "devrs-env".to_string(),
            image_tag: "latest".to_string(),
            // Define a default mount: current host directory -> /code in container.
            mounts: vec![
                config::MountConfig {
                    host: host_path, // The current directory on the host.
                    container: "/code".to_string(), // Target path inside the container.
                    readonly: false, // Make it read-write by default.
                },
            ],
            ports: vec![], // No default port mappings.
            env_vars: Default::default(), // No default environment variables.
            // Set the default working directory inside the container to match the mount point.
            default_workdir: "/code".to_string(),
        },
        // Use default (empty) settings for other config sections.
        blueprints: Default::default(),
        application_defaults: Default::default(),
    })
}


// --- Unit Tests ---
// Focus on argument parsing and the default config generation logic.
// Testing `handle_shell` requires extensive mocking.
#[cfg(test)]
mod tests {
    use super::*;

    /// Test parsing arguments with the optional --name flag.
    #[test]
    fn test_shell_args_parsing() {
        // Simulate `devrs env shell` (no args)
        let args_default = ShellArgs::try_parse_from(["shell"]).unwrap();
        assert!(args_default.name.is_none()); // Name should be None by default.

        // Simulate `devrs env shell --name custom-core-env`
        let args_named =
            ShellArgs::try_parse_from(["shell", "--name", "custom-core-env"]).unwrap();
        // Name should be parsed correctly.
        assert_eq!(args_named.name, Some("custom-core-env".to_string()));
    }

    /// Test the logic for creating the minimal default configuration fallback.
    #[tokio::test]
    async fn test_create_minimal_default_config() {
        // Execute the function to get the default config.
        let result = create_minimal_default_config().await;
        // Expect it to succeed.
        assert!(result.is_ok());
        let cfg = result.unwrap();

        // Verify the hardcoded default image name and tag.
        assert_eq!(cfg.core_env.image_name, "devrs-env");
        assert_eq!(cfg.core_env.image_tag, "latest");
        // Verify the default working directory.
        assert_eq!(cfg.core_env.default_workdir, "/code");
        // Verify that exactly one mount point was created.
        assert_eq!(cfg.core_env.mounts.len(), 1);
        // Verify the container path for the default mount.
        assert_eq!(cfg.core_env.mounts[0].container, "/code");
        // Verify the host path is not empty (it should be the test execution directory).
        assert!(!cfg.core_env.mounts[0].host.is_empty());
        // Verify default ports and env vars are empty.
        assert!(cfg.core_env.ports.is_empty());
        assert!(cfg.core_env.env_vars.is_empty());
    }

    // Note: Testing `handle_shell`'s main logic requires mocking:
    // 1. `config::load_config` -> To simulate both successful load and failure/fallback cases.
    // 2. `common::docker::lifecycle::ensure_core_env_running` -> To simulate container being ready/created.
    // 3. `common::docker::interaction::exec_in_container` -> To simulate the shell session and its exit.
    // 4. `env::current_dir` (if testing default config generation thoroughly).
    #[tokio::test]
    #[ignore] // Requires mocking.
    async fn test_handle_shell_logic_with_config() {
        // --- Mocking Setup (Conceptual) ---
        // Mock `config::load_config` -> Ok(valid_config_with_image_details)
        // Mock `ensure_core_env_running` -> Ok(false) // Simulate container already exists.
        // Mock `exec_in_container` -> Ok(0) // Simulate successful shell exit.

        // --- Execution ---
        let args = ShellArgs { name: None }; // Use default container name.
        let result = handle_shell(args).await;

        // --- Assertions ---
        assert!(result.is_ok());
        // Verify (via mocks) that `ensure_core_env_running` and `exec_in_container` were called
        // with arguments derived from the mocked `valid_config`.
    }

    #[tokio::test]
    #[ignore] // Requires mocking.
    async fn test_handle_shell_logic_without_config() {
        // --- Mocking Setup (Conceptual) ---
        // Mock `config::load_config` -> Err(...) // Simulate config load failure.
        // Mock `ensure_core_env_running` -> Ok(true) // Simulate container needed creation.
        // Mock `exec_in_container` -> Ok(0) // Simulate successful shell exit.
        // Mock `env::current_dir` if needed for precise default config verification.

        // --- Execution ---
        let args = ShellArgs { name: None }; // Use default container name.
        let result = handle_shell(args).await;

        // --- Assertions ---
        assert!(result.is_ok());
        // Verify (via mocks) that `ensure_core_env_running` and `exec_in_container` were called
        // with arguments derived from the *default* config (e.g., image "devrs-env:latest", workdir "/code").
    }
} 
