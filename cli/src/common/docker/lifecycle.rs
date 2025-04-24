//! # DevRS Docker Lifecycle Operations
//!
//! File: cli/src/common/docker/lifecycle.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!

//! ## Overview
//!
//! This module provides functions dedicated to managing the **lifecycle** of Docker
//! containers. It focuses on actions that change the state of a container, such as
//! starting, stopping, and removing them. It also includes specialized logic for
//! ensuring the core DevRS development environment container is running (`ensure_core_env_running`).
//!
//! ## Architecture
//!
//! Key functions implemented:
//! - **`start_container`**: Takes a container name/ID and attempts to start it if it's stopped. Handles the "already running" case gracefully (Docker 304 response).
//! - **`stop_container`**: Takes a container name/ID and attempts to stop it gracefully within an optional timeout, falling back to a force kill if necessary. Handles the "already stopped" case gracefully (Docker 304 response).
//! - **`remove_container`**: Takes a container name/ID and attempts to remove it. Includes a `force` flag. If `force` is false, it first checks if the container is running and returns an error if it is. Handles the "not found" case gracefully.
//! - **`ensure_core_env_running`**: A higher-level function specifically for the core DevRS environment. It checks if the designated container exists and is running. If not, it automatically creates and/or starts it based on the application configuration (`config::Config`). This involves calling `state::container_exists`, `state::container_running`, `operations::run_container`, and `start_container` as needed.
//!
//! These functions rely on helpers from sibling modules (`connect`, `state`, `operations`)
//! and map Docker API errors to consistent `DevrsError` types.
//!
//! ## Usage
//!
//! These functions are primarily used by command handlers that need to manipulate container states.
//!
//! ```rust
//! use crate::common::docker::lifecycle;
//! use crate::core::config; // Assuming config is loaded
//! use crate::core::error::Result;
//!
//! # async fn run_example() -> Result<()> {
//! let cfg = config::Config::default(); // Example config
//! let my_app_container = "my-app-instance";
//! let core_env_name = format!("{}-instance", cfg.core_env.image_name);
//!
//! // Ensure the core dev environment is running (creates/starts if needed)
//! let was_created = lifecycle::ensure_core_env_running(&core_env_name, &cfg).await?;
//! if was_created {
//!     println!("Core environment container was newly created.");
//! }
//!
//! // Start a specific application container (if stopped)
//! lifecycle::start_container(my_app_container).await?;
//!
//! // Stop the application container with a 5-second timeout
//! lifecycle::stop_container(my_app_container, Some(5)).await?;
//!
//! // Remove the application container (only if stopped, unless force=true)
//! lifecycle::remove_container(my_app_container, false).await?;
//! # Ok(())
//! # }
//! ```
//!
use crate::core::{
    config, // Application configuration structure
    error::{DevrsError, Result}, // Standard Result and custom Error types
};
use anyhow::{anyhow, Context}; // For error context wrapping
use bollard::container::{
    // Options structs for lifecycle operations
    RemoveContainerOptions,
    StartContainerOptions,
    StopContainerOptions,
};
// Use Duration for delays if needed later
use tracing::{debug, error, info, instrument, warn}; // Logging utilities

// Import functions from sibling modules needed for lifecycle operations.
use super::connect::connect_docker; // Get Docker client connection
use super::operations; // Access operations like run_container (needed for ensure_core_env)
use super::state::{container_exists, container_running}; // Check container status before actions

/// Starts a stopped Docker container identified by its name or ID.
///
/// If the container is already running, this function treats it as a success (idempotent).
/// It handles the Docker API's 304 (Not Modified) response code gracefully.
///
/// # Arguments
///
/// * `name_or_id` - The name or ID of the container to start.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if the container was successfully started or was already running.
///
/// # Errors
///
/// * `DevrsError::ContainerNotFound` - If the specified container does not exist (Docker 404).
/// * `DevrsError::DockerApi` - For other errors during communication with the Docker daemon.
#[instrument(skip(name_or_id), fields(container = %name_or_id))] // Tracing span
pub async fn start_container(name_or_id: &str) -> Result<()> {
    // Establish connection to Docker daemon.
    let docker = connect_docker().await?;
    info!("Attempting to start container '{}'...", name_or_id); // Log action

    // Call the bollard start_container function.
    match docker
        .start_container(name_or_id, None::<StartContainerOptions<String>>) // No specific start options used
        .await
    {
        // Start successful.
        Ok(_) => {
            info!("Container '{}' started successfully.", name_or_id);
            Ok(())
        }
        // Handle specific Docker error codes.
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 304, .. // 304 means "Not Modified", i.e., already running.
        }) => {
            info!("Container '{}' was already started.", name_or_id);
            Ok(()) // Treat as success.
        }
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, .. // 404 means "Not Found".
        }) => {
            warn!(
                "Start failed because container '{}' was not found.",
                name_or_id
            );
            // Return our specific ContainerNotFound error.
            Err(anyhow!(DevrsError::ContainerNotFound {
                name: name_or_id.to_string()
            }))
        }
        // Handle any other Docker API errors.
        Err(e) => {
            error!("Failed to start container '{}': {:?}", name_or_id, e);
            // Wrap the error and provide context.
            Err(anyhow!(DevrsError::DockerApi { source: e })
                .context(format!("Failed to start container '{}'", name_or_id)))
        }
    }
}

/// Stops a running Docker container identified by its name or ID.
///
/// Attempts a graceful shutdown using SIGTERM, waiting for the specified `timeout_secs`
/// before Docker forcibly kills the container (usually with SIGKILL).
/// If the container is already stopped, this function treats it as a success (idempotent).
///
/// # Arguments
///
/// * `name_or_id` - The name or ID of the container to stop.
/// * `timeout_secs` - An optional duration (in seconds) to wait for graceful shutdown.
///                    If `None`, Docker's default timeout (typically 10 seconds) is used.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if the container was successfully stopped or was already stopped.
///
/// # Errors
///
/// * `DevrsError::ContainerNotFound` - If the specified container does not exist (Docker 404).
/// * `DevrsError::DockerApi` - For other errors during communication with the Docker daemon.
#[instrument(skip(name_or_id, timeout_secs), fields(container = %name_or_id))] // Tracing span
pub async fn stop_container(name_or_id: &str, timeout_secs: Option<u32>) -> Result<()> {
    // Establish connection to Docker daemon.
    let docker = connect_docker().await?;
    // Prepare the options struct for the stop_container API call.
    // Convert the Option<u32> timeout to the i64 expected by bollard.
    let options = timeout_secs.map(|t| StopContainerOptions { t: t as i64 });
    // Log the action with the specified timeout.
    info!(
        "Attempting to stop container '{}' (Timeout: {:?} seconds)...",
        name_or_id,
        timeout_secs.map_or_else(|| "default (10)".to_string(), |t| t.to_string()) // Log default clearly
    );

    // Call the bollard stop_container function.
    match docker.stop_container(name_or_id, options).await {
        // Stop successful.
        Ok(_) => {
            info!("Container '{}' stopped successfully.", name_or_id);
            Ok(())
        }
        // Handle specific Docker error codes.
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 304, .. // 304 means "Not Modified", i.e., already stopped.
        }) => {
            info!("Container '{}' was already stopped.", name_or_id);
            Ok(()) // Treat as success.
        }
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, .. // 404 means "Not Found".
        }) => {
            warn!(
                "Stop failed because container '{}' was not found.",
                name_or_id
            );
            // Return our specific ContainerNotFound error.
            Err(anyhow!(DevrsError::ContainerNotFound {
                name: name_or_id.to_string()
            }))
        }
        // Handle any other Docker API errors.
        Err(e) => {
            error!("Failed to stop container '{}': {:?}", name_or_id, e);
            // Wrap the error and provide context.
            Err(anyhow!(DevrsError::DockerApi { source: e })
                .context(format!("Failed to stop container '{}'", name_or_id)))
        }
    }
}

/// Removes a Docker container identified by its name or ID.
///
/// Includes an option to force removal. If `force` is `false`, the function first
/// checks if the container is running using `container_running`. If it is running,
/// an error (`DevrsError::ContainerRunning`) is returned to prevent accidental removal
/// of active containers. If `force` is `true`, the check is skipped, and removal is
/// attempted directly (Docker may still prevent removal of running containers depending
/// on its state).
///
/// # Arguments
///
/// * `name_or_id` - The name or ID of the container to remove.
/// * `force` - If `true`, skip the running check and attempt removal directly. If `false`, check if running first.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if the container was successfully removed or if it did not exist.
///
/// # Errors
///
/// * `DevrsError::ContainerRunning` - If `force` is `false` and the container is found to be running.
/// * `DevrsError::ContainerNotFound` - Although handled gracefully (returns Ok), underlying Docker 404 might propagate if initial state check fails unexpectedly.
/// * `DevrsError::DockerOperation` - For conflicts preventing removal (Docker 409).
/// * `DevrsError::DockerApi` - For other errors during communication with the Docker daemon or during the initial state check.
#[instrument(skip(name_or_id, force), fields(container = %name_or_id))] // Tracing span
pub async fn remove_container(name_or_id: &str, force: bool) -> Result<()> {
    // Establish connection to Docker daemon.
    let docker = connect_docker().await?;

    // --- Pre-Removal Check (if force=false) ---
    if !force {
        // Check if the container is currently running.
        match container_running(name_or_id).await {
            // Container is running, cannot remove without force.
            Ok(true) => {
                warn!(
                    "Attempted to remove running container '{}' without --force.",
                    name_or_id
                );
                // Return specific error indicating it's running.
                return Err(anyhow!(DevrsError::ContainerRunning {
                    name: name_or_id.to_string()
                }));
            }
            // Container exists but is stopped, proceed with removal.
            Ok(false) => {
                debug!(
                    "Container '{}' is stopped, proceeding with removal.",
                    name_or_id
                );
            }
            // Check if the error during the running check was specifically "Not Found".
            Err(e)
                if e.downcast_ref::<DevrsError>().map_or(false, |err| {
                    matches!(err, DevrsError::ContainerNotFound { .. })
                }) =>
            {
                // Container doesn't exist, so removal is effectively successful.
                info!("Container '{}' not found, no removal needed.", name_or_id);
                return Ok(()); // Return Ok immediately.
            }
            // An unexpected error occurred during the running status check.
            Err(e) => {
                return Err(e).context("Failed to check container status before removal");
            }
        }
    }
    // If force=true, skip the running check.

    // --- Removal Attempt ---
    info!(
        "Attempting to remove container '{}' (Force: {})...",
        name_or_id, force
    );
    // Prepare options for the remove_container API call.
    let options = Some(RemoveContainerOptions {
        force, // Apply the force flag as determined.
        v: false, // Do not remove associated anonymous volumes by default. Change if needed.
        link: false, // Deprecated option, set to false.
    });

    // Call the bollard remove_container function.
    match docker.remove_container(name_or_id, options).await {
        // Removal successful.
        Ok(_) => {
            info!("Container '{}' removed successfully.", name_or_id);
            Ok(())
        }
        // Handle specific Docker error codes.
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, .. // 404 means "Not Found".
        }) => {
            // Should ideally be caught by the pre-check if force=false,
            // but handle here for completeness or if force=true.
            info!(
                "Container '{}' not found during removal attempt.",
                name_or_id
            );
            Ok(()) // Treat as success if the goal is absence.
        }
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 409, // 409 means "Conflict" (e.g., running and force=false, or other removal conflict).
            message,          // Docker often provides a reason.
        }) => {
            error!(
                "Conflict removing container '{}': {}. Use --force or check Docker logs.",
                name_or_id, message
            );
            // Return our specific DockerOperation error.
            Err(anyhow!(DevrsError::DockerOperation(format!(
                "Conflict removing container '{}': {}. Try using --force or investigate Docker state.", name_or_id, message
            ))))
        }
        // Handle any other Docker API errors.
        Err(e) => {
            error!("Failed to remove container '{}': {:?}", name_or_id, e);
            // Wrap the error and provide context.
            Err(anyhow!(DevrsError::DockerApi { source: e })
                .context(format!("Failed to remove container '{}'", name_or_id)))
        }
    }
}

/// Ensures the core DevRS environment container exists and is running.
///
/// This function is a high-level utility specifically for managing the persistent
/// core development environment container used by `devrs env shell` and `devrs env exec`.
/// It checks the container's status and performs the necessary actions (create, start)
/// using the details provided in the `config::Config`.
///
/// # Arguments
///
/// * `name` - The expected name of the core environment container (e.g., "devrs-core-env-instance").
/// * `cfg` - A reference to the loaded `config::Config` containing details about the core environment image, mounts, ports, etc.
///
/// # Returns
///
/// * `Result<bool>` - `Ok(true)` if the container was newly created during this call, `Ok(false)` if the container already existed (and was potentially started).
///
/// # Errors
///
/// Returns an error if:
/// - The container needs to be created but the configured image (`cfg.core_env.image_name`) is not found locally (`DevrsError::ImageNotFound`).
/// - Container creation fails due to Docker API errors or conflicts (`DevrsError::DockerOperation`, `DevrsError::DockerApi`).
/// - An existing, stopped container fails to start (`DevrsError::DockerApi`).
/// - The container fails to reach a running state after creation/start attempts (`DevrsError::DockerOperation`).
#[instrument(skip(name, cfg), fields(container = %name))] // Tracing span
pub async fn ensure_core_env_running(name: &str, cfg: &config::Config) -> Result<bool> {
    // Construct the full image name:tag string from configuration.
    let image_name_with_tag = format!("{}:{}", cfg.core_env.image_name, cfg.core_env.image_tag);
    // Flag to track if we created the container in this function call.
    let mut created = false;

    // --- Check Existence and State ---
    // Check if the container exists.
    if !container_exists(name).await? {
        // Container does not exist, needs to be created.
        info!(
            "Core env container '{}' not found. Creating from image '{}'...",
            name, image_name_with_tag
        );
        created = true; // Mark that we are creating it.

        // Call the run_container operation (from the sibling 'operations' module).
        // Configure it for the core env: detached, persistent (no auto-remove).
        match operations::run_container(
            &image_name_with_tag,                 // Image name from config.
            name,                                 // Target container name.
            &cfg.core_env.ports,                  // Ports from config.
            &cfg.core_env.mounts,                 // Mounts from config.
            &cfg.core_env.env_vars,               // Env vars from config.
            Some(&cfg.core_env.default_workdir), // Workdir from config.
            true,                                 // detached = true (run in background).
            false,                                // auto_remove = false (persist).
            None,                                 // No command override, use image default.
        )
        .await // Await the async creation/start operation.
        {
            // Container created and started successfully.
            Ok(()) => {
                info!("Successfully created and started container '{}'.", name);
                // Optional brief pause if needed for services inside container to fully start.
                // tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            // Handle potential errors during run_container.
            Err(e) => {
                // Specifically check if the error indicates the required image wasn't found.
                if let Some(docker_err) = e.downcast_ref::<bollard::errors::Error>() {
                    if let bollard::errors::Error::DockerResponseServerError {
                        status_code: 404, // Bollard often uses 404 for image not found on create.
                        ..
                    } = docker_err
                    {
                        // Return our specific ImageNotFound error with guidance.
                        return Err(anyhow!(DevrsError::ImageNotFound {
                            name: image_name_with_tag.clone()
                        })
                        .context(format!(
                            "Core image '{}' was not found by Docker. Run 'devrs env build'.",
                            image_name_with_tag
                        )));
                    }
                    // Check for container name conflict (409) - less likely here due to prior check, but possible race condition.
                    else if let bollard::errors::Error::DockerResponseServerError {
                        status_code: 409, // Check status code for conflict.
                        message,
                    } = docker_err
                    {
                        error!("Conflict creating container '{}': {}. It might already exist unexpectedly.", name, message);
                        // Return our specific DockerOperation error.
                        return Err(anyhow!(DevrsError::DockerOperation(format!(
                            "Conflict creating container '{}': {}. It may already exist.",
                            name, message
                        ))));
                    }
                }
                // For any other error during creation/start, propagate it with context.
                return Err(e).context(format!(
                    "Failed to create core environment container '{}' from image '{}'. Check Docker daemon status and image name.",
                    name, image_name_with_tag
                ));
            }
        }
    }
    // Container exists, now check if it's running.
    else if !container_running(name).await? {
        // Container exists but is stopped, needs to be started.
        info!("Container '{}' exists but is stopped. Starting...", name);
        start_container(name) // Call the start function from this module.
            .await
            .with_context(|| format!("Failed to start stopped container '{}'", name))?;
        info!("Successfully started container '{}'.", name);
        // Optional brief pause.
        // tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    // Container exists and is already running.
    else {
        info!("Container '{}' is already running.", name);
    }

    // --- Final Verification ---
    // After create/start attempts, verify the container is actually running now.
    if !container_running(name).await? {
        // If still not running, something went wrong.
        return Err(anyhow!(DevrsError::DockerOperation(format!(
            "Container '{}' failed to reach running state after create/start attempt.",
            name
        ))));
    }

    // Return Ok, indicating if the container was newly created or just ensured running.
    Ok(created)
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    // Note: Testing these lifecycle functions requires mocking the Docker API interactions
    // provided by the `connect`, `state`, and `operations` modules.
    // Tests should cover scenarios like:
    // - Starting a stopped container.
    // - Starting an already running container (should be Ok).
    // - Starting a non-existent container (should return ContainerNotFound error).
    // - Stopping a running container.
    // - Stopping an already stopped container (should be Ok).
    // - Stopping a non-existent container (should return ContainerNotFound error).
    // - Removing a stopped container.
    // - Removing a non-existent container (should be Ok).
    // - Removing a running container with force=false (should return ContainerRunning error).
    // - Removing a running container with force=true (mock should simulate success/failure).
    // - ensure_core_env_running when container doesn't exist (verify run_container call).
    // - ensure_core_env_running when container stopped (verify start_container call).
    // - ensure_core_env_running when container running (verify no action needed).
    // - ensure_core_env_running when image missing (verify ImageNotFound error).

    /// Placeholder test to ensure the module compiles.
    #[test]
    fn placeholder_lifecycle_test() {
        assert!(true);
    }
}
