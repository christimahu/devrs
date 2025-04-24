//! # DevRS Container Stop Handler
//!
//! File: cli/src/commands/container/stop.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs container stop` subcommand, which stops
//! one or more running Docker containers identified by their names or IDs.
//! It provides a user-friendly interface to Docker's container stopping mechanism,
//! supporting concurrent stopping of multiple containers and customized timeouts.
//!
//! ## Architecture
//!
//! The implementation follows these steps:
//!
//! 1. Parse command-line arguments (`StopArgs`) using `clap`, capturing one or more container names/IDs and the optional `--time` (timeout) flag.
//! 2. For each specified container name/ID, spawn an asynchronous Tokio task to handle the stop operation.
//! 3. Within each task, call the shared `common::docker::lifecycle::stop_container` utility function, passing the name/ID and the timeout value.
//! 4. The utility function interacts with the Docker API to request a graceful stop, potentially falling back to a force kill after the timeout.
//! 5. Collect the results from all spawned tasks using `futures_util::future::join_all`.
//! 6. Process the results, specifically treating "ContainerNotFound" and "already stopped" scenarios as successful outcomes for this command's intent.
//! 7. Report overall success or list any containers that failed to stop along with the corresponding errors.
//!
//! ## Usage
//!
//! ```bash
//! # Stop a single container (uses default 10s timeout)
//! devrs container stop my-running-app
//!
//! # Stop multiple containers
//! devrs container stop app1 app2 container_id_123
//!
//! # Specify the timeout (in seconds) before force-killing
//! devrs container stop --time 5 my-slow-container
//! # Shorthand:
//! devrs container stop -t 3 app3
//! ```
//!
//! The command attempts to stop containers in parallel when multiple names are provided.
//!
use crate::{
    common::docker, // Access shared Docker utilities, specifically lifecycle::stop_container.
    core::error::Result, // Standard Result type for error handling.
};
use anyhow::Context; // For adding context to errors.
use clap::Parser; // For parsing command-line arguments.
use futures_util::future::join_all; // For running multiple async stop tasks concurrently.
use tracing::{error, info, warn}; // Logging framework utilities.

/// # Container Stop Arguments (`StopArgs`)
///
/// Defines the command-line arguments accepted by the `devrs container stop` subcommand.
/// Uses the `clap` crate for parsing and validation.
#[derive(Parser, Debug)]
#[command(about = "Stop one or more running application containers")]
pub struct StopArgs {
    /// One or more names or IDs of the running application containers to stop.
    /// At least one container name or ID must be provided.
    #[arg(required = true, num_args = 1..)] // Require at least one value, allow multiple.
    container_names_or_ids: Vec<String>,

    /// Optional: Specifies the duration (in seconds) to wait for the container(s)
    /// to stop gracefully before Docker forcibly kills them (e.g., with SIGKILL).
    /// Defaults to 10 seconds if not provided.
    #[arg(long, short, default_value = "10")] // Define as `--time` or `-t`, with a default.
    time: u32,
    // TODO: Consider adding an --all flag later? It would need careful implementation
    //       to filter out the core dev environment container before stopping others.
}

/// # Handle Container Stop Command (`handle_stop`)
///
/// The main asynchronous handler function for the `devrs container stop` command.
/// It iterates through the list of provided container names/IDs and attempts
/// to stop each one concurrently using Tokio tasks.
///
/// ## Workflow:
/// 1.  Logs the command execution details, including names and timeout.
/// 2.  Converts the `time` argument into an `Option<u32>` for the underlying Docker call.
/// 3.  Initializes an empty vector `stop_tasks` for asynchronous operations.
/// 4.  Loops through each `name` provided in `args.container_names_or_ids`.
/// 5.  For each `name`, spawns a new Tokio task:
///     * The task calls `common::docker::lifecycle::stop_container` with the `name` and `timeout`.
///     * Handles the `Result` from `stop_container`:
///         * If `Ok(())`, prints a success message and returns `Ok(name)`.
///         * If `Err` represents `DevrsError::ContainerNotFound`, logs a warning but considers it success for the stop intent, returning `Ok(name)`.
///         * If `Err` indicates the container was already stopped (detected via specific error message check), logs info and returns `Ok(name)`.
///         * For any other `Err`, logs the error and returns `Err((name, error))`.
/// 6.  Waits for all spawned tasks to complete using `join_all`.
/// 7.  Iterates through the results, collecting any errors (`Err((name, error))`) into `failed_stops`.
/// 8.  Reports the final status:
///     * If `failed_stops` is empty, returns `Ok(())`.
///     * If failures occurred, prints an error summary and returns the first error encountered, wrapped with context.
///
/// ## Arguments
///
/// * `args`: The parsed `StopArgs` struct containing the list of container names/IDs and the timeout duration.
///
/// ## Returns
///
/// * `Result<()>`: Returns `Ok(())` if all specified containers were successfully stopped or were already stopped/absent.
/// * `Err`: Returns an `Err` if stopping failed for one or more containers (and provides details about the first failure).
pub async fn handle_stop(args: StopArgs) -> Result<()> {
    // Log entry point and arguments.
    info!(
        "Handling container stop command (Names: {:?}, Time: {})",
        args.container_names_or_ids, args.time
    );

    // Prepare the timeout value for the Docker API call (expects Option<u32>).
    let timeout = Some(args.time);
    // Vector to store handles for the asynchronous stop tasks.
    let mut stop_tasks = Vec::new();

    // Iterate through each container name/ID provided by the user.
    for name in &args.container_names_or_ids {
        // Clone the name for moving into the async task.
        let name = name.clone();
        // Spawn a Tokio task for each stop operation.
        stop_tasks.push(tokio::spawn(async move {
            // Call the shared Docker utility function to stop the container.
            match docker::lifecycle::stop_container(&name, timeout).await { //
                // Stop operation was successful according to Docker.
                Ok(()) => {
                    println!("Stopped container '{}'", name); // Inform the user.
                    Ok(name) // Return Ok with the name for success tracking.
                }
                // Check if the error indicates the container was not found.
                Err(e) if e.downcast_ref::<crate::core::error::DevrsError>().map_or(false, |de| matches!(de, crate::core::error::DevrsError::ContainerNotFound { .. })) => {
                    // Container didn't exist. Log this, but consider it success for the 'stop' intent.
                    warn!("Container '{}' not found.", name);
                    Ok(name) // Return Ok, as the desired state (container stopped/absent) is achieved.
                }
                 // Check if the error indicates the container was already stopped.
                 // This relies on the specific error message structure returned by our stop_container helper.
                 Err(e) if e.downcast_ref::<crate::core::error::DevrsError>().map_or(false, |de| matches!(de, crate::core::error::DevrsError::DockerOperation(msg) if msg.contains("already stopped"))) => {
                    // Container was already stopped. Log this as info.
                    info!("Container '{}' was already stopped.", name);
                     Ok(name) // Return Ok, as the desired state is achieved.
                 }
                // Any other error occurred during the stop attempt.
                Err(e) => {
                    // Log the failure.
                    error!("Failed to stop container '{}': {:?}", name, e);
                    // Return Err containing the name and the specific error for detailed reporting.
                    Err((name, e))
                }
            }
        }));
    }

    // Wait for all the spawned stop tasks to complete.
    let results = join_all(stop_tasks).await;

    // Collect details of any container stops that failed.
    let mut failed_stops = Vec::new();
    // Process the results from each completed task.
    for result in results {
        match result {
            // Task finished successfully, but the inner stop_container operation returned an error.
            Ok(Err((name, e))) => {
                failed_stops.push((name, e)); // Add the name and error to the list.
            }
            // The Tokio task itself failed (e.g., panicked). This is unexpected.
            Err(e) => {
                // Log this unexpected task failure. We don't know which container it was for.
                error!("Container stop task failed unexpectedly: {}", e);
                // Potentially add a generic failure entry if needed? For now, just log.
            }
            // Task finished successfully and the inner stop_container returned Ok
            // (successful stop, or container was not found/already stopped).
            Ok(Ok(_)) => {
                // No action needed for success cases.
            }
        }
    }

    // --- Report Final Status ---
    // Check if any stops failed.
    if failed_stops.is_empty() {
        // All specified containers were successfully stopped or were already in a stopped/absent state.
        info!("Successfully processed stop request for all specified containers.");
        Ok(()) // Return Ok for overall success.
    } else {
        // Report the errors encountered for specific containers.
        eprintln!("\nErrors occurred during container stop:");
        for (name, err) in &failed_stops {
            eprintln!("- {}: {}", name, err); // Print each failed container and the error.
        }
        // Return the first error encountered, adding context.
        let first_error = failed_stops.remove(0).1; // Retrieve the anyhow::Error.
        Err(first_error).context(format!(
            "Failed to stop {} container(s)",
            // The total number of failures is the original length of the failed_stops list.
            failed_stops.len() + 1
        ))
    }
}


// --- Unit Tests ---
// Focus on argument parsing for the `stop` command. Testing the handler logic
// requires mocking the Docker API interaction within `docker::lifecycle::stop_container`.
#[cfg(test)]
mod tests {
    use super::*;

    /// Test parsing with multiple container names and a custom timeout flag.
    #[test]
    fn test_stop_args_parsing() {
        // Simulate `devrs container stop c1 c2 -t 5`
        let args = StopArgs::try_parse_from(&[
            "stop", // Command name context for clap.
            "c1",   // First positional argument (name/ID).
            "c2",   // Second positional argument.
            "-t", "5", // Timeout flag with value.
        ])
        .unwrap(); // Expect parsing to succeed.

        // Verify the container names/IDs were captured.
        assert_eq!(args.container_names_or_ids, vec!["c1", "c2"]);
        // Verify the custom timeout value was parsed.
        assert_eq!(args.time, 5);
    }

    /// Test parsing with default timeout.
    #[test]
    fn test_stop_args_parsing_default_time() {
         // Simulate `devrs container stop c1` (no time flag)
         let args = StopArgs::try_parse_from(&[ "stop", "c1" ]).unwrap();
         assert_eq!(args.container_names_or_ids, vec!["c1"]);
         // Verify the default timeout value is used.
         assert_eq!(args.time, 10);
    }


    /// Test that the command fails parsing if no container names/IDs are provided.
    #[test]
    fn test_stop_args_requires_name() {
        // Simulate `devrs container stop` (with no names)
        let result = StopArgs::try_parse_from(&["stop"]);
        // Expect an error because at least one name/ID is required.
        assert!(result.is_err(), "Should fail without container names");
    }

    // Note: Integration tests for `handle_stop` would involve mocking the
    // `docker::lifecycle::stop_container` function to simulate different
    // outcomes (success, not found, already stopped, other errors)
    // and verifying that `handle_stop` processes these results correctly,
    // including parallel execution and error aggregation/reporting.
}
