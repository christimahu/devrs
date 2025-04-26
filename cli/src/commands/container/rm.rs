//! # DevRS Container Removal Handler
//!
//! File: cli/src/commands/container/rm.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs container rm` subcommand, which removes
//! one or more Docker containers identified by their names or IDs. It acts as a
//! wrapper around the underlying Docker container removal functionality, adding
//! support for removing multiple containers concurrently and handling specific
//! error conditions gracefully.
//!
//! ## Architecture
//!
//! The implementation follows these steps:
//!
//! 1. Parse command-line arguments using `clap`, capturing one or more container names/IDs and the optional `--force` flag.
//! 2. For each specified container name/ID, spawn an asynchronous Tokio task to handle its removal.
//! 3. Within each task, call the shared `common::docker::lifecycle::remove_container` utility function, passing the name/ID and the force flag.
//! 4. The utility function interacts with the Docker API to perform the removal, handling checks for running containers if `force` is false.
//! 5. Collect the results from all spawned tasks using `futures_util::future::join_all`.
//! 6. Process the results, treating "ContainerNotFound" errors as successful removals (the container is already gone).
//! 7. Report overall success or list any containers that failed to be removed along with the corresponding errors.
//!
//! ## Usage
//!
//! ```bash
//! # Remove a single container
//! devrs container rm my-container
//!
//! # Remove multiple containers
//! devrs container rm container1 container2 some_other_container_id
//!
//! # Force remove a container (e.g., if it's stopped but won't remove normally, or potentially running)
//! devrs container rm --force my-stuck-container
//! ```
//!
//! The command attempts to remove containers in parallel for efficiency when multiple names are provided.
//!
use crate::{
    common::docker, // Access shared Docker utilities, specifically lifecycle::remove_container.
    core::error::Result, // Standard Result type for error handling.
};
use anyhow::Context; // For adding context to errors.
use clap::Parser; // For parsing command-line arguments.
use futures_util::future::join_all; // For running multiple async removal tasks concurrently.
use tracing::{error, info, warn}; // Logging framework utilities.

/// # Container Remove Arguments (`RmArgs`)
///
/// Defines the command-line arguments accepted by the `devrs container rm` subcommand.
/// Uses the `clap` crate for parsing and validation.
#[derive(Parser, Debug)]
#[command(about = "Remove one or more application containers")]
pub struct RmArgs {
    /// One or more names or IDs of the application containers to remove.
    /// At least one container name or ID must be provided.
    #[arg(required = true, num_args = 1..)] // Require at least one value, allow multiple.
    container_names_or_ids: Vec<String>,

    /// Optional: Force the removal of the container(s).
    /// If a container is running, this flag is typically required to remove it
    /// (Docker usually sends SIGKILL). If the container is stopped, this flag
    /// might help bypass certain removal conflicts but is often not needed.
    #[arg(long, short)] // Define as `--force` or `-f`.
    force: bool,
    // TODO: Consider adding a `--volumes` or `-v` flag in the future to optionally remove associated anonymous volumes.
}

/// # Handle Container Remove Command (`handle_rm`)
///
/// The main asynchronous handler function for the `devrs container rm` command.
/// It iterates through the list of provided container names/IDs and attempts
/// to remove each one concurrently using Tokio tasks.
///
/// ## Workflow:
/// 1.  Logs the command execution details.
/// 2.  Initializes an empty vector `removal_tasks` to hold the asynchronous removal operations.
/// 3.  Loops through each `name` provided in `args.container_names_or_ids`.
/// 4.  For each `name`, spawns a new Tokio task:
///     * The task calls `common::docker::lifecycle::remove_container` with the `name` and `force` flag.
///     * Handles the `Result` from `remove_container`:
///         * If `Ok(())`, prints a success message and returns `Ok(name)`.
///         * If `Err` represents `DevrsError::ContainerNotFound`, logs a warning but treats it as success for removal intent, returning `Ok(name)`.
///         * For any other `Err`, logs the error and returns `Err((name, error))`.
/// 5.  Waits for all spawned tasks to complete using `join_all`.
/// 6.  Iterates through the results of the tasks:
///     * Collects any errors (`Err((name, error))`) returned by the tasks into `failed_removals`.
///     * Logs errors for tasks that panicked or were cancelled.
/// 7.  Reports the final status:
///     * If `failed_removals` is empty, returns `Ok(())`.
///     * If there were failures, prints an error summary listing the failed containers and their errors, then returns the first encountered error wrapped with context.
///
/// ## Arguments
///
/// * `args`: The parsed `RmArgs` struct containing the list of container names/IDs and the `force` flag.
///
/// ## Returns
///
/// * `Result<()>`: Returns `Ok(())` if all specified containers were successfully removed or were already absent.
/// * `Err`: Returns an `Err` if the removal failed for one or more containers (and provides details about the first failure).
pub async fn handle_rm(args: RmArgs) -> Result<()> {
    // Log entry and arguments.
    info!(
        "Handling container rm command (Names: {:?}, Force: {})",
        args.container_names_or_ids, args.force
    );

    // Create a vector to hold the handles of the spawned asynchronous tasks.
    let mut removal_tasks = Vec::new();

    // Iterate through each container name/ID provided by the user.
    for name in &args.container_names_or_ids {
        // Clone the name and force flag to move them into the async task.
        let name = name.clone();
        let force = args.force;
        // Spawn a new asynchronous task for each removal.
        removal_tasks.push(tokio::spawn(async move {
            // Call the shared Docker utility function to remove the container.
            // This internally uses the Docker API's remove_container operation.
            match docker::lifecycle::remove_container(&name, force).await { //
                // Removal was successful according to the Docker API.
                Ok(()) => {
                    println!("Removed container '{}'", name); // Inform the user.
                    Ok(name) // Return Ok with the name for success tracking.
                }
                // Check if the error indicates the container was not found.
                Err(e)
                    if e.downcast_ref::<crate::core::error::DevrsError>() // Safely attempt to downcast the anyhow::Error
                        .is_some_and(|de| { // If downcast succeeds, check the DevrsError variant.
                            // Match specifically against the ContainerNotFound variant.
                            matches!(de, crate::core::error::DevrsError::ContainerNotFound { .. })
                        }) =>
                {
                    // Container didn't exist. Log this, but consider it success for the 'rm' operation's intent.
                    warn!("Container '{}' not found.", name);
                    Ok(name) // Return Ok, as the desired state (container absent) is achieved.
                }
                // Any other error occurred during removal.
                Err(e) => {
                    // Log the failure.
                    error!("Failed to remove container '{}': {:?}", name, e);
                    // Return Err containing the name and the specific error for reporting.
                    Err((name, e))
                }
            }
        }));
    }

    // Wait for all the spawned removal tasks to complete.
    let results = join_all(removal_tasks).await;

    // Collect details of any removals that failed.
    let mut failed_removals = Vec::new();
    // Process the results from each completed task.
    for result in results {
        match result {
            // Task finished successfully, but the inner operation returned an error.
            Ok(Err((name, e))) => {
                failed_removals.push((name, e)); // Add the name and error to the failure list.
            }
            // The Tokio task itself failed (e.g., panicked).
            Err(e) => {
                // Log this unexpected task failure. We don't know which container it was for.
                error!("Container removal task failed unexpectedly: {}", e);
            }
            // Task finished successfully and the inner operation also returned Ok (successful removal or not found).
            Ok(Ok(_)) => {
                // No action needed for successful removals.
            }
        }
    }

    // --- Report Final Status ---
    // Check if any removals failed.
    if failed_removals.is_empty() {
        // All removals were successful (or containers were already gone).
        info!("Successfully processed removal for all specified containers.");
        Ok(()) // Return Ok for overall success.
    } else {
        // At least one removal failed. Report the errors.
        eprintln!("\nErrors occurred during container removal:");
        // Print each failed container and the reason.
        for (name, err) in &failed_removals {
            eprintln!("- {}: {}", name, err);
        }
        // To return a single error, take the first one from the list.
        let first_error = failed_removals.remove(0).1; // Get the anyhow::Error.
        // Return the first error, adding context about the overall failure.
        Err(first_error).context(format!(
            "Failed to remove {} container(s)",
            // The number of failures is the original length of failed_removals.
            failed_removals.len() + 1
        ))
    }
}


// --- Unit Tests ---
// Focus on argument parsing for the `rm` command. Testing the handler logic
// requires mocking the Docker API interaction within `docker::lifecycle::remove_container`.
#[cfg(test)]
mod tests {
    use super::*;

    /// Test parsing with multiple container names and the force flag.
    #[test]
    fn test_rm_args_parsing() {
        // Simulate `devrs container rm c1 c2 -f`
        let args = RmArgs::try_parse_from([
            "rm", // Command name context for clap
            "c1", // First positional argument
            "c2", // Second positional argument
            "-f", // Force flag
        ])
        .unwrap();
        // Verify the container names were captured correctly in the vector.
        assert_eq!(args.container_names_or_ids, vec!["c1", "c2"]);
        // Verify the force flag was parsed as true.
        assert!(args.force);
    }

    /// Test that the command fails parsing if no container names/IDs are provided.
    #[test]
    fn test_rm_args_requires_name() {
        // Simulate `devrs container rm` (with no names)
        let result = RmArgs::try_parse_from(["rm"]);
        // Expect an error because at least one name/ID is required.
        assert!(result.is_err(), "Should fail without container names");
    }

    // Note: Integration tests for `handle_rm` would involve mocking the
    // `docker::lifecycle::remove_container` function to simulate different
    // outcomes (success, not found, running without force, other errors)
    // and verifying that `handle_rm` processes these results correctly,
    // including parallel execution and error aggregation.
}
