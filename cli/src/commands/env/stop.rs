//! # DevRS Environment Stop Handler
//!
//! File: cli/src/commands/env/stop.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs env stop` subcommand. Its purpose is to
//! stop the running **core development environment** container, allowing it
//! to be restarted later (via `devrs env shell`) or removed (via `devrs env prune`).
//!
//! ## Architecture
//!
//! The command flow involves these steps:
//! 1. Parse command-line arguments (`StopArgs`) using `clap`, capturing the optional `--name` override and the `--time` timeout value.
//! 2. Load the DevRS configuration (`core::config`) to determine the default name of the core environment container if `--name` is not provided.
//! 3. Determine the final target container name.
//! 4. Call the shared Docker utility function `common::docker::lifecycle::stop_container`, passing the container name and the timeout value. This function handles the Docker API call to stop the container gracefully (or force kill after timeout).
//! 5. Process the result from the utility function, specifically handling "ContainerNotFound" and "already stopped" scenarios as successful outcomes for this command's intent.
//! 6. Report success or failure to the user.
//!
//! ## Usage
//!
//! ```bash
//! # Stop the default core environment container (uses default 10s timeout)
//! devrs env stop
//!
//! # Stop a specifically named core environment container
//! devrs env stop --name my-custom-env-instance
//!
//! # Specify the timeout (in seconds) before force-killing
//! devrs env stop --time 5
//! # Shorthand:
//! devrs env stop -t 3
//! ```
//!
//! This command only affects the core development environment container, not application-specific containers managed by `devrs container stop`.
//!
use crate::{
    common::docker::{self}, // Access shared Docker utilities (specifically lifecycle::stop_container).
    core::{
        config, // Access configuration loading.
        error::{DevrsError, Result}, // Standard Result type and custom errors.
    },
};
use anyhow::Context; // For adding context to errors.
use clap::Parser; // For parsing command-line arguments.
use tracing::{debug, info, warn}; // Logging framework utilities.

/// # Environment Stop Arguments (`StopArgs`)
///
/// Defines the command-line arguments accepted by the `devrs env stop` subcommand.
/// Uses the `clap` crate for parsing and validation.
#[derive(Parser, Debug)]
#[command(about = "Stop the core development environment container")]
pub struct StopArgs {
    /// Optional: Specifies the exact name of the core environment container to stop.
    /// If omitted, the default name (derived from the `core_env.image_name` in the configuration,
    /// typically `<image_name>-instance`) is used.
    #[arg(long)] // Define as `--name <NAME>`.
    name: Option<String>,

    /// Optional: Specifies the duration (in seconds) to wait for the container
    /// to stop gracefully after receiving the stop signal, before Docker forcibly
    /// kills it (e.g., with SIGKILL).
    /// Defaults to 10 seconds if not provided.
    #[arg(long, short, default_value = "10")] // Define as `--time` or `-t`, with a default.
    time: u32,
}

/// # Handle Environment Stop Command (`handle_stop`)
///
/// The main asynchronous handler function for the `devrs env stop` command.
/// It identifies the target core environment container and attempts to stop it
/// using the shared Docker utility function.
///
/// ## Workflow:
/// 1.  Logs the start and parsed arguments.
/// 2.  Loads the DevRS configuration to get core environment details (needed for default container name).
/// 3.  Determines the target container name: uses `--name` if provided, otherwise generates the default name using `get_core_env_container_name`.
/// 4.  Prepares the timeout value (`Some(args.time)`) for the Docker API call.
/// 5.  Calls `common::docker::lifecycle::stop_container` with the determined container name and timeout.
/// 6.  Processes the `Result` from `stop_container`:
///     * If `Ok(())`, the container was successfully stopped or was already stopped. Prints success message, returns `Ok(())`.
///     * If `Err` represents `DevrsError::ContainerNotFound`, logs a warning but considers the goal achieved (container is absent/stopped), prints a "not found" message, and returns `Ok(())`.
///     * If any other `Err` occurs, propagates the error up, adding context.
///
/// ## Arguments
///
/// * `args`: The parsed `StopArgs` struct containing the optional container `name` and `time` arguments.
///
/// ## Returns
///
/// * `Result<()>`: Returns `Ok(())` if the container was successfully stopped, was already stopped, or was not found.
/// * `Err`: Returns an `Err` if config loading fails or if the Docker API call fails for reasons other than "not found" or "already stopped".
pub async fn handle_stop(args: StopArgs) -> Result<()> {
    info!("Handling env stop command..."); // Log entry point.
    debug!("Stop args: {:?}", args); // Log arguments if debug enabled.

    // 1. Load configuration - needed to determine the default container name.
    let cfg = config::load_config().context("Failed to load DevRS configuration")?;

    // 2. Determine the target core environment container name.
    let container_name = args.name.clone().unwrap_or_else(|| {
        // If --name not specified, generate the default name.
        let default_name = get_core_env_container_name(&cfg);
        debug!("No specific name provided, using default: {}", default_name);
        default_name
    });

    // 3. Call the shared Docker utility function to stop the container.
    println!( // Inform user which container is being targeted.
        "Attempting to stop core env container '{}'...",
        container_name
    );
    // Prepare timeout value as Option<u32> required by the utility function.
    let timeout = Some(args.time);

    // Attempt to stop the container.
    match docker::lifecycle::stop_container(&container_name, timeout).await { //
        // Stop command succeeded or container was already stopped (stop_container handles this).
        Ok(()) => {
            println!(
                "✅ Container '{}' stopped successfully or was already stopped.",
                container_name
            );
            Ok(()) // Overall command success.
        }
        // Handle specific errors gracefully.
        Err(e) => {
            // Check if the error was 'ContainerNotFound'.
            if e.downcast_ref::<DevrsError>().is_some_and(|de| {
                matches!(de, DevrsError::ContainerNotFound { .. })
            }) {
                // If not found, log warning and inform user, but return Ok for the command itself.
                warn!("Container '{}' not found, nothing to stop.", container_name);
                println!("Container '{}' not found.", container_name);
                Ok(()) // Desired state (stopped/absent) is achieved.
            }
            // Note: The stop_container helper also handles the Docker 304 "already stopped" case internally and returns Ok.
            // Therefore, we only need to handle "not found" specifically here.
            else {
                // For any other type of error, propagate it up with added context.
                Err(e).context(format!("Failed to stop container '{}'", container_name))
            }
        }
    }
}

/// # Get Core Environment Container Name (`get_core_env_container_name`)
/// Helper function to derive the default container name based on the configured image name.
/// Appends "-instance" to the image name.
fn get_core_env_container_name(cfg: &config::Config) -> String {
    // Simple string formatting to construct the default name.
    format!("{}-instance", cfg.core_env.image_name)
}


// --- Unit Tests ---
// Focus on argument parsing for the `stop` command. Testing the handler logic
// requires mocking config loading and Docker API interactions.
#[cfg(test)]
mod tests {
    use super::*;

    /// Test parsing arguments, including the optional container name and timeout.
    #[test]
    fn test_stop_args_parsing() {
        // Simulate `devrs env stop --name custom-env -t 5`
        let args_named = StopArgs::try_parse_from(["stop", "--name", "custom-env", "-t", "5"])
            .expect("Parsing named args failed");
        assert_eq!(args_named.name, Some("custom-env".to_string())); // Check name.
        assert_eq!(args_named.time, 5); // Check custom timeout.

         // Simulate `devrs env stop` (no optional args)
        let args_default = StopArgs::try_parse_from(["stop"]).expect("Parsing default args failed");
        assert!(args_default.name.is_none()); // Name should be None.
        assert_eq!(args_default.time, 10); // Check default timeout.
    }

    // Note: Testing `handle_stop`'s logic requires mocking:
    // 1. `config::load_config` -> To provide a core env image name for deriving the default container name.
    // 2. `common::docker::lifecycle::stop_container` -> To simulate different outcomes:
    //    - Ok(()) for successful stop or already stopped.
    //    - Err(DevrsError::ContainerNotFound) for the not found case.
    //    - Other Err variants for general Docker API failures.
    // Then, tests can verify that `handle_stop` correctly processes these results and returns Ok/Err appropriately.
    #[tokio::test]
    #[ignore] // Requires mocking.
    async fn test_handle_stop_logic_running() {
        // Mock config -> Ok(config_with_name)
        // Mock stop_container("derived-name", Some(10)) -> Ok(())

        let args = StopArgs { name: None, time: 10 };
        let result = handle_stop(args).await;
        assert!(result.is_ok());
        // Verify stop_container mock was called.
    }

    #[tokio::test]
    #[ignore] // Requires mocking.
    async fn test_handle_stop_logic_not_found() {
        // Mock config -> Ok(config_with_name)
        // Mock stop_container("derived-name", Some(10)) -> Err(anyhow!(DevrsError::ContainerNotFound { name: ... }))

        let args = StopArgs { name: None, time: 10 };
        let result = handle_stop(args).await;
        // Should still return Ok even if container not found.
        assert!(result.is_ok());
        // Verify stop_container mock was called.
    }
}
