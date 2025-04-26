//! # DevRS Environment Pruning Handler
//!
//! File: cli/src/commands/env/prune.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs env prune` subcommand. Its purpose is to
//! clean up unused resources specifically related to the **core DevRS development
//! environment**. Currently, this focuses on removing **stopped** core environment
//! containers that may accumulate over time (e.g., after rebuilds if removal failed).
//!
//! ## Architecture
//!
//! The command flow involves these steps:
//! 1. Parse command-line arguments (`PruneArgs`) using `clap`, mainly the `--force` flag which acts as confirmation.
//! 2. Load the DevRS configuration (`core::config`) to determine the naming pattern for the core environment container(s) (e.g., `<image_name>-instance`).
//! 3. Call the shared Docker utility `common::docker::state::list_containers` with the `all=true` flag to get a list of *all* containers (running and stopped).
//! 4. Filter this list to identify only those containers that:
//!    * Match the core environment naming pattern.
//!    * Are **not** currently running (e.g., have status 'created', 'exited', 'dead').
//! 5. If no containers match the criteria, print a message and exit successfully.
//! 6. If matching stopped containers are found, print the list of containers that will be removed.
//! 7. Check if the `--force` flag was provided. If *not*, print a warning and exit (acting as a dry run/confirmation step). **Note:** Currently, no interactive prompt is implemented.
//! 8. If `--force` was provided, proceed with removal:
//!    * Spawn asynchronous Tokio tasks for each container identified for pruning.
//!    * Each task calls `common::docker::lifecycle::remove_container` with `force=false` (since the containers are already verified to be stopped).
//!    * Collect results using `join_all`.
//! 9. Report overall success or list any containers that failed removal.
//!
//! ## Usage
//!
//! ```bash
//! # Perform a dry run (lists containers that *would* be pruned)
//! devrs env prune
//!
//! # Prune stopped core environment containers (requires confirmation via --force)
//! devrs env prune --force
//! # Shorthand:
//! devrs env prune -f
//! ```
//!
//! **Important:** This command currently only targets stopped core environment *containers*. It does not prune the core environment image or associated volumes.
//!
use crate::{
    common::docker::{self}, // Access shared Docker utilities (list_containers, remove_container).
    core::{config, error::Result}, // Standard config loading and Result type.
};
use anyhow::Context; // For adding context to errors.
use clap::Parser; // For parsing command-line arguments.
use futures_util::future::join_all; // For running multiple async removal tasks concurrently.
use tracing::{debug, error, info, warn}; // Logging framework utilities.

/// # Environment Prune Arguments (`PruneArgs`)
///
/// Defines the command-line arguments accepted by the `devrs env prune` subcommand.
/// Uses the `clap` crate for parsing and validation.
#[derive(Parser, Debug)]
#[command(
    about = "Clean up unused core DevRS environment Docker resources",
    long_about = "Removes stopped containers associated with the core DevRS environment.\n\
                  WARNING: This currently only removes containers, not images or volumes."
)]
pub struct PruneArgs {
    /// Confirms the removal operation. Without this flag, the command performs a dry run,
    /// listing containers that would be removed but not actually removing them.
    /// **Note:** Since this command currently only targets *stopped* containers,
    /// this flag primarily acts as a confirmation rather than enabling removal of running containers.
    #[arg(long, short)] // Define as `--force` or `-f`.
    force: bool,
    // TODO: Consider adding flags like `--images` or `--volumes` in the future if
    //       env-specific pruning of these resources is desired.
}

/// # Handle Environment Prune Command (`handle_prune`)
///
/// The main asynchronous handler function for the `devrs env prune` command.
/// It identifies and removes stopped containers associated with the core DevRS
/// development environment.
///
/// ## Workflow:
/// 1.  Logs the command start and the value of the `force` flag.
/// 2.  Loads the DevRS configuration to get the `core_env.image_name` used to derive the container naming pattern (e.g., `<image_name>-instance`).
/// 3.  Lists *all* Docker containers (running and stopped) using `common::docker::state::list_containers`.
/// 4.  Filters the list to find containers that match the naming pattern AND are in a non-running state ('created', 'exited', 'dead', etc.).
/// 5.  If the filtered list is empty, prints a message and exits successfully.
/// 6.  If containers are found, prints the names/IDs of those targeted for removal.
/// 7.  Checks the `args.force` flag. If `false`, prints a warning explaining that `--force` is needed to proceed and exits successfully (simulating a dry run).
/// 8.  If `args.force` is `true`, proceeds to remove the targeted containers concurrently:
///     * Spawns a Tokio task for each container ID.
///     * Each task calls `common::docker::lifecycle::remove_container` with `force=false` (since we've already filtered for stopped containers).
///     * Collects results using `join_all`.
/// 9.  Processes the results, reporting overall success or listing any containers that failed removal.
///
/// ## Arguments
///
/// * `args`: The parsed `PruneArgs` struct containing the `force` flag.
///
/// ## Returns
///
/// * `Result<()>`: Returns `Ok(())` if the prune operation completes successfully (either nothing to prune, dry run completed, or forced removal succeeded for all targets).
/// * `Err`: Returns an `Err` if configuration loading fails, listing containers fails, or if forced removal fails for any targeted container.
pub async fn handle_prune(args: PruneArgs) -> Result<()> {
    info!("Handling env prune command (Force: {})...", args.force); // Log entry.

    // 1. Load config to get the core env image name for pattern matching.
    let cfg = config::load_config().context("Failed to load DevRS configuration")?;
    // Assume container name follows the pattern "<image_name>-instance".
    let core_env_name_prefix = format!("{}-instance", cfg.core_env.image_name);
    // Define the pattern for filtering (used later).
    debug!(
        "Targeting stopped containers starting with: {}",
        core_env_name_prefix
    );

    // 2. List *all* containers (running and stopped) to filter locally.
    let all_containers = docker::state::list_containers(true, None) // `all=true`, no docker-side filters needed.
        .await
        .context("Failed to list Docker containers")?;

    // 3. Filter the list to find stopped containers matching the core env pattern.
    let containers_to_prune: Vec<_> = all_containers
        .into_iter()
        .filter(|c| {
            // Check if any name matches the prefix.
            let name_matches = c.names.as_ref().is_some_and(|names| {
                names
                    .iter()
                    .any(|n| n.trim_start_matches('/').starts_with(&core_env_name_prefix))
            });
            // Check if the container state is *not* one considered "running".
            let is_stopped = !matches!(
                c.state.as_deref(), // Get container state string (e.g., "running", "exited").
                Some("running") | Some("restarting") | Some("paused") // Explicitly list running states.
            );
            // Keep if both name matches and it's stopped.
            name_matches && is_stopped
        })
        .collect();

    // 4. Handle cases based on whether containers were found.
    if containers_to_prune.is_empty() {
        println!("No stopped core DevRS environment containers found to prune.");
        return Ok(()); // Nothing to do, successful exit.
    }

    // 5. List containers identified for pruning.
    println!("The following stopped core environment containers will be removed:");
    for c in &containers_to_prune {
        // Extract short ID and names for display.
        let id = c.id.as_deref().unwrap_or("unknown-id")[..12].to_string();
        let names = c
            .names
            .as_ref()
            .map_or_else(|| "N/A".to_string(), |n| n.join(", "));
        println!("  - {} ({})", id, names);
    }

    // 6. Check for confirmation via --force flag.
    // Currently, this acts purely as confirmation, as we only target stopped containers.
    if !args.force {
        // Warn the user and exit if --force is not provided (dry run).
        warn!("Prune operation aborted. Re-run with --force to confirm removal.");
        // Note: An interactive prompt could be added here using crates like `dialoguer`.
        // if !Confirm::new().with_prompt("Proceed with removal?").interact()? { ... }
        return Ok(()); // Treat cancellation/dry run as successful command execution.
    }

    // 7. Proceed with removal if --force was given.
    info!("Proceeding with prune...");
    // Vector to hold async task handles.
    let mut removal_tasks = Vec::new();
    // Spawn a task for each container to be removed.
    for container_summary in containers_to_prune {
        if let Some(id) = container_summary.id {
            // Ensure container has an ID.
            // Spawn the async removal task.
            removal_tasks.push(tokio::spawn(async move {
                // Call the shared remove function. Pass force=false, as we know it's stopped.
                // This prevents accidentally trying to force-remove a container that *was* stopped
                // but somehow got restarted between the list and remove calls (unlikely but possible).
                match docker::lifecycle::remove_container(&id, false).await {
                    //
                    Ok(()) => {
                        println!("Removed container '{}'", id); // Inform user of success.
                        Ok(id) // Return Ok for success tracking.
                    }
                    Err(e) => {
                        // Log failure and return Err for aggregation.
                        error!("Failed to remove container '{}': {:?}", id, e);
                        Err((id, e)) // Return tuple of (id, error).
                    }
                }
            }));
        }
    }

    // Wait for all removal tasks to complete.
    let results = join_all(removal_tasks).await;

    // 8. Collect and report results.
    let mut failed_removals = Vec::new();
    for result in results {
        match result {
            // Task completed, but inner remove operation failed.
            Ok(Err((id, e))) => failed_removals.push((id, e)),
            // Task itself failed (e.g., panicked).
            Err(e) => error!("Container removal task failed unexpectedly: {}", e),
            // Task completed successfully (inner operation returned Ok).
            Ok(Ok(_)) => {} // Successful removal.
        }
    }

    // Check if any removals failed.
    if failed_removals.is_empty() {
        info!("Successfully pruned all targeted containers.");
        println!("✅ Prune complete.");
        Ok(()) // Overall success.
    } else {
        // Report failures.
        eprintln!("\nErrors occurred during container prune:");
        for (id, err) in &failed_removals {
            eprintln!("- {}: {}", id, err);
        }
        // Return the first error encountered, adding context.
        let first_error = failed_removals.remove(0).1;
        Err(first_error).context(format!(
            "Failed to prune {} container(s)",
            failed_removals.len() + 1 // Original number of failures.
        ))
    }
}

// --- Unit Tests ---
// Focus on argument parsing. Testing handler logic requires mocking.
#[cfg(test)]
mod tests {
    use super::*;

    /// Test parsing default arguments (no flags).
    #[test]
    fn test_prune_args_parsing() {
        // Simulate `devrs env prune`
        let args = PruneArgs::try_parse_from(["prune"]).unwrap();
        // Default value for `force` should be false.
        assert!(!args.force);
    }

    /// Test parsing with the `--force` flag (or `-f`).
    #[test]
    fn test_prune_args_parsing_force() {
        // Simulate `devrs env prune --force`
        let args_force = PruneArgs::try_parse_from(["prune", "--force"]).unwrap();
        // The `force` flag should be true.
        assert!(args_force.force);
        // Simulate `devrs env prune -f`
        let args_force_short = PruneArgs::try_parse_from(["prune", "-f"]).unwrap();
        assert!(args_force_short.force);
    }

    // Note: Testing the `handle_prune` function's logic requires mocking:
    // 1. `config::load_config` -> To provide a known core env image name pattern.
    // 2. `common::docker::state::list_containers` -> To return a controlled list of containers
    //    with various states and names, including some that should be filtered out.
    // 3. `common::docker::lifecycle::remove_container` -> To simulate success/failure of removal.
    // Then, tests could verify:
    // - Correct filtering of containers based on state and name.
    // - Correct behavior with and without the `--force` flag (dry run vs. actual removal calls).
    // - Correct aggregation and reporting of errors if `remove_container` fails.
}
