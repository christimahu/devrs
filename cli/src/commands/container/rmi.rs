//! # DevRS Image Removal Handler
//!
//! File: cli/src/commands/container/rmi.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs container rmi` subcommand, which removes
//! one or more Docker images from the local image cache, identified by their
//! names or IDs. It provides a convenient wrapper around the underlying Docker
//! image removal functionality, enabling concurrent removal of multiple images
//! and handling common error scenarios.
//!
//! ## Architecture
//!
//! The implementation follows these steps:
//!
//! 1. Parse command-line arguments using `clap`, capturing one or more image names/IDs and the optional `--force` flag.
//! 2. For each specified image name/ID, spawn an asynchronous Tokio task to handle its removal.
//! 3. Within each task, call the shared `common::docker::images::remove_image` utility function, passing the name/ID and the force flag.
//! 4. The utility function interacts with the Docker API to perform the removal, handling potential conflicts like the image being in use.
//! 5. Collect the results from all spawned tasks using `futures_util::future::join_all`.
//! 6. Process the results, treating "ImageNotFound" errors as successful removals (the image is already gone).
//! 7. Report overall success or list any images that failed to be removed along with the corresponding errors.
//!
//! ## Usage
//!
//! ```bash
//! # Remove a single image by tag
//! devrs container rmi my-app:latest
//!
//! # Remove an image by ID (prefix)
//! devrs container rmi 1a2b3c4d5e6f
//!
//! # Remove multiple images
//! devrs container rmi image1:v1 image2:v2 some_other_image
//!
//! # Force remove an image (may be needed if tagged by multiple names or used by stopped containers)
//! devrs container rmi --force my-image:latest
//! ```
//!
//! The `--force` flag attempts to remove the image even if it might have dependencies
//! or be used by stopped containers. It generally does *not* force removal of images
//! currently used by *running* containers unless specific Docker daemon settings allow it.
//!
use crate::{
    common::docker, // Access shared Docker utilities, specifically images::remove_image.
    core::error::Result, // Standard Result type for error handling.
};
use anyhow::Context; // For adding context to errors.
use clap::Parser; // For parsing command-line arguments.
use futures_util::future::join_all; // For running multiple async removal tasks concurrently.
use tracing::{error, info, warn}; // Logging framework utilities.

/// # Remove Image Arguments (`RmiArgs`)
///
/// Defines the command-line arguments accepted by the `devrs container rmi` subcommand.
/// Uses the `clap` crate for parsing and validation.
#[derive(Parser, Debug)]
#[command(about = "Remove one or more application Docker images")]
pub struct RmiArgs {
    /// One or more names (e.g., "my-app:latest", "repo/img:v1") or IDs (full or prefix)
    /// of the application Docker images to remove from the local cache.
    /// At least one image name or ID must be provided.
    #[arg(required = true, num_args = 1..)] // Require at least one value, allow multiple.
    image_names_or_ids: Vec<String>,

    /// Optional: Force the removal of the image(s).
    /// This may be necessary if the image is tagged multiple times or is used as a
    /// parent layer for other images, or if it's associated with stopped containers.
    /// It typically does *not* force removal if the image is currently used by a
    /// *running* container unless specific Docker daemon configurations are set.
    #[arg(long, short)] // Define as `--force` or `-f`.
    force: bool,
}

/// # Handle Remove Image Command (`handle_rmi`)
///
/// The main asynchronous handler function for the `devrs container rmi` command.
/// It iterates through the list of provided image names/IDs and attempts
/// to remove each one concurrently using Tokio tasks.
///
/// ## Workflow:
/// 1.  Logs the command execution details.
/// 2.  Initializes an empty vector `removal_tasks` to hold the asynchronous removal operations.
/// 3.  Loops through each `name` provided in `args.image_names_or_ids`.
/// 4.  For each `name`, spawns a new Tokio task:
///     * The task calls `common::docker::images::remove_image` with the `name` and `force` flag.
///     * Handles the `Result` from `remove_image`:
///         * If `Ok(())`, prints a success message and returns `Ok(name)`.
///         * If `Err` represents `DevrsError::ImageNotFound`, logs a warning but treats it as success for removal intent, returning `Ok(name)`.
///         * For any other `Err` (like `ImageInUse`), logs the error and returns `Err((name, error))`.
/// 5.  Waits for all spawned tasks to complete using `join_all`.
/// 6.  Iterates through the results of the tasks:
///     * Collects any errors (`Err((name, error))`) returned by the tasks into `failed_removals`.
///     * Logs errors for tasks that panicked or were cancelled.
/// 7.  Reports the final status:
///     * If `failed_removals` is empty, returns `Ok(())`.
///     * If there were failures, prints an error summary listing the failed images and their errors, then returns the first encountered error wrapped with context.
///
/// ## Arguments
///
/// * `args`: The parsed `RmiArgs` struct containing the list of image names/IDs and the `force` flag.
///
/// ## Returns
///
/// * `Result<()>`: Returns `Ok(())` if all specified images were successfully removed or were already absent.
/// * `Err`: Returns an `Err` if the removal failed for one or more images (e.g., image in use and `--force` not specified or insufficient).
pub async fn handle_rmi(args: RmiArgs) -> Result<()> {
    // Log entry point and arguments.
    info!(
        "Handling container rmi command (Names: {:?}, Force: {})",
        args.image_names_or_ids, args.force
    );

    // Vector to store handles for the asynchronous removal tasks.
    let mut removal_tasks = Vec::new();
    // Iterate through each image name/ID provided by the user.
    for name in &args.image_names_or_ids {
        // Clone name and force flag for moving into the async task.
        let name = name.clone();
        let force = args.force;
        // Spawn a Tokio task for each image removal.
        removal_tasks.push(tokio::spawn(async move {
            // Call the shared Docker utility function to remove the image.
            match docker::images::remove_image(&name, force).await { //
                // Image removal successful.
                Ok(()) => {
                    println!("Removed image '{}'", name); // Inform the user.
                    Ok(name) // Return Ok with the name for success tracking.
                }
                // Check if the error specifically indicates the image was not found.
                Err(e)
                    if e.downcast_ref::<crate::core::error::DevrsError>() // Safely attempt to downcast anyhow::Error.
                        .is_some_and(|de| { // Check the specific DevrsError variant if downcast succeeds.
                            matches!(de, crate::core::error::DevrsError::ImageNotFound { .. })
                        }) =>
                {
                    // Image didn't exist. Log this, but consider it successful for the 'rmi' intent.
                    warn!("Image '{}' not found.", name);
                    Ok(name) // Return Ok as the desired state (image absent) is achieved.
                }
                // Any other error occurred (e.g., ImageInUse, Docker API error).
                Err(e) => {
                    // Log the failure.
                    error!("Failed to remove image '{}': {:?}", name, e);
                    // Return Err containing the name and the specific error for detailed reporting.
                    Err((name, e))
                }
            }
        }));
    }

    // Wait for all spawned removal tasks to finish.
    let results = join_all(removal_tasks).await;

    // Collect details of any image removals that failed.
    let mut failed_removals = Vec::new();
    // Process the results from each completed task.
    for result in results {
        match result {
            // Task completed successfully, but the underlying remove_image operation returned an error.
            Ok(Err((name, e))) => {
                failed_removals.push((name, e)); // Add the name and error to the list.
            }
            // The Tokio task itself failed (e.g., panicked). This is unexpected.
            Err(e) => {
                // Log this unexpected task failure. We don't know which image it was for.
                error!("Image removal task failed unexpectedly: {}", e);
            }
            // Task completed successfully and the underlying remove_image returned Ok (successful removal or not found).
            Ok(Ok(_)) => {
                // No action needed for success.
            }
        }
    }

    // --- Report Final Status ---
    // Check if there were any failures during the removal process.
    if failed_removals.is_empty() {
        // All specified images were successfully removed or were already absent.
        info!("Successfully processed removal for all specified images.");
        Ok(()) // Return Ok to indicate overall success.
    } else {
        // Report the errors encountered for specific images.
        eprintln!("\nErrors occurred during image removal:");
        for (name, err) in &failed_removals {
            eprintln!("- {}: {}", name, err); // Print each failed image and the error message.
        }
        // To provide a single error return value, take the first error from the list.
        let first_error = failed_removals.remove(0).1; // Retrieve the anyhow::Error.
        // Return the first error, adding context about the overall failure.
        Err(first_error).context(format!(
            "Failed to remove {} image(s)",
            // The total number of failures is the original length of the failed_removals list.
            failed_removals.len() + 1
        ))
    }
}


// --- Unit Tests ---
// Focus on argument parsing for the `rmi` command. Testing the handler logic
// requires mocking the Docker API interaction within `docker::images::remove_image`.
#[cfg(test)]
mod tests {
    use super::*;

    /// Test parsing with multiple image names/IDs and the force flag.
    #[test]
    fn test_rmi_args_parsing() {
        // Simulate `devrs container rmi img1:latest img2:v1.0 --force`
        let args = RmiArgs::try_parse_from([
            "rmi",         // Command name context for clap.
            "img1:latest", // First positional argument.
            "img2:v1.0",   // Second positional argument.
            "--force",     // Optional force flag.
        ])
        .unwrap();
        // Verify the image names were captured correctly.
        assert_eq!(args.image_names_or_ids, vec!["img1:latest", "img2:v1.0"]);
        // Verify the force flag was parsed.
        assert!(args.force);
    }

    /// Test that the command fails parsing if no image names/IDs are provided.
    #[test]
    fn test_rmi_args_requires_name() {
        // Simulate `devrs container rmi` (with no image names)
        let result = RmiArgs::try_parse_from(["rmi"]);
        // Expect an error because at least one image name/ID is required.
        assert!(result.is_err(), "Should fail without image names");
    }

    // Note: Integration tests for `handle_rmi` would involve mocking the
    // `docker::images::remove_image` function to simulate different
    // outcomes (success, not found, image in use, other errors)
    // and verifying that `handle_rmi` processes these results correctly,
    // including parallel execution and error aggregation.
}
