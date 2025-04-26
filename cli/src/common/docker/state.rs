//! # DevRS Docker State Querying
//!
//! File: cli/src/common/docker/state.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!

//! ## Overview
//!
//! This module provides utility functions focused on **querying the state and metadata**
//! of Docker containers without causing any modifications. It allows other parts of
//! the application to determine if containers exist, check their running status,
//! retrieve detailed inspection information (like IP addresses, mounts, config),
//! and list containers based on specific criteria.
//!
//! ## Architecture
//!
//! The module centralizes state-querying logic using the `bollard` crate:
//! - **`container_exists`**: A boolean check utilizing `inspect_container` and specifically handling the 404 (Not Found) error case.
//! - **`inspect_container`**: Wraps the `bollard` `inspect_container` call, returning the full `ContainerInspectResponse` or a specific `DevrsError::ContainerNotFound` error.
//! - **`container_running`**: Determines the running status by inspecting the container and checking the `State.Status` field. Handles the "not found" case gracefully by returning `Ok(false)`.
//! - **`list_containers`**: Wraps the `bollard` `list_containers` call, allowing filtering by status (all/running only) and other Docker API filters.
//!
//! All functions use the shared `connect::connect_docker` helper and map relevant Docker API errors to the application's standard `Result` and `DevrsError` types.
//!
//! ## Usage
//!
//! These functions are used extensively by command handlers to make decisions based on the current Docker state.
//!
//! ```rust
//! use crate::common::docker::state;
//! use crate::core::error::Result;
//! use std::collections::HashMap;
//!
//! # async fn run_example() -> Result<()> {
//! let container_name = "my-web-app";
//!
//! // Check if the container exists before trying to interact
//! if state::container_exists(container_name).await? {
//!     println!("Container '{}' exists.", container_name);
//!
//!     // Check if it's actually running
//!     if state::container_running(container_name).await? {
//!         println!("Container '{}' is running.", container_name);
//!
//!         // Get detailed info if needed
//!         let details = state::inspect_container(container_name).await?;
//!         println!("IP Address: {}", details.network_settings.unwrap_or_default().ip_address.unwrap_or_default());
//!     } else {
//!         println!("Container '{}' exists but is stopped.", container_name);
//!     }
//! } else {
//!     println!("Container '{}' does not exist.", container_name);
//! }
//!
//! // List all containers (running and stopped) with a specific label
//! let mut filters = HashMap::new();
//! filters.insert("label".to_string(), vec!["project=my-project".to_string()]);
//! let project_containers = state::list_containers(true, Some(filters)).await?; // all=true
//! println!("Found {} containers for 'my-project'.", project_containers.len());
//! # Ok(())
//! # }
//! ```
//!
use crate::core::error::{DevrsError, Result}; // Use standard Result and custom Error
use anyhow::anyhow; // For error context wrapping
use bollard::{
    container::{InspectContainerOptions, ListContainersOptions}, // Options for inspect/list
    models::{ContainerInspectResponse, ContainerStateStatusEnum, ContainerSummary}, // Response types
                                                                                    // Docker client is obtained via connect_docker
};
use std::collections::HashMap; // For list_containers filters map
use tracing::{debug, error, info, instrument, warn}; // Logging utilities

// Import the shared connection helper from the sibling module.
use super::connect::connect_docker;

/// Checks if a Docker container exists locally by name or ID.
///
/// This function uses `inspect_container` and interprets a "Not Found" (404)
/// response from the Docker API as `false`, while other errors are propagated.
///
/// # Arguments
///
/// * `name_or_id` - The name or ID of the container to check.
///
/// # Returns
///
/// * `Result<bool>` - `Ok(true)` if the container exists, `Ok(false)` if it does not (404 error),
///                    or an `Err` for other Docker API communication issues.
///
/// # Errors
///
/// Returns `DevrsError::DockerApi` wrapped in `anyhow::Error` for non-404 Docker errors during inspection.
#[instrument(skip(name_or_id), fields(container = %name_or_id))] // Tracing span
pub async fn container_exists(name_or_id: &str) -> Result<bool> {
    // Establish connection to Docker daemon.
    let docker = connect_docker().await?;
    debug!("Checking existence for container: {}", name_or_id); // Log action

    // Attempt to inspect the container.
    match docker
        .inspect_container(name_or_id, None::<InspectContainerOptions>) // No specific inspect options needed
        .await
    {
        // Inspection succeeded, meaning the container exists.
        Ok(_) => {
            debug!("Container '{}' exists.", name_or_id);
            Ok(true)
        }
        // Inspection failed with a 404 error, meaning the container does not exist.
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {
            debug!("Container '{}' does not exist (404).", name_or_id);
            Ok(false)
        }
        // Inspection failed for another reason (e.g., Docker daemon unavailable, permissions).
        Err(e) => {
            error!(
                "Failed to inspect container '{}' during existence check: {:?}",
                name_or_id, e
            );
            // Propagate the error, wrapped appropriately.
            Err(anyhow!(DevrsError::DockerApi { source: e })
                .context(format!("Failed to inspect container '{}'", name_or_id)))
        }
    }
}

/// Inspects a container by name or ID to retrieve detailed information.
///
/// Fetches the full JSON response from the Docker `inspect` API endpoint for containers.
///
/// # Arguments
///
/// * `name_or_id` - The name or ID of the container to inspect.
///
/// # Returns
///
/// * `Result<ContainerInspectResponse>` - A struct containing the detailed container inspection information.
///
/// # Errors
///
/// * `DevrsError::ContainerNotFound` - If the container doesn't exist (maps Docker 404).
/// * `DevrsError::DockerApi` - For other errors during communication with the Docker daemon.
#[instrument(skip(name_or_id), fields(container = %name_or_id))] // Tracing span
pub async fn inspect_container(name_or_id: &str) -> Result<ContainerInspectResponse> {
    // Establish connection to Docker daemon.
    let docker = connect_docker().await?;
    debug!("Inspecting container: {}", name_or_id); // Log action

    // Call the bollard inspect_container function.
    docker
        .inspect_container(name_or_id, None::<InspectContainerOptions>) // No specific options needed
        .await
        // Map potential errors to our custom error types.
        .map_err(|e| match e {
            // Handle the specific case where the container is not found (404).
            bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            } => {
                warn!(
                    "Inspection failed because container '{}' was not found.",
                    name_or_id
                );
                // Create our specific ContainerNotFound error.
                anyhow!(DevrsError::ContainerNotFound {
                    name: name_or_id.to_string()
                })
            }
            // Handle all other Docker API errors generically.
            _ => {
                error!("Failed to inspect container '{}': {:?}", name_or_id, e);
                // Wrap the original bollard error in our DockerApi error.
                anyhow!(DevrsError::DockerApi { source: e })
                    .context(format!("Failed to inspect container '{}'", name_or_id))
            }
        })
}

/// Checks if a container identified by name or ID is currently in the 'running' state.
///
/// This function first inspects the container using `inspect_container`. If the container
/// exists, it checks the `State.Status` field in the response. It handles the case where
/// the container is not found by returning `Ok(false)`.
///
/// # Arguments
///
/// * `name_or_id` - The name or ID of the container to check.
///
/// # Returns
///
/// * `Result<bool>` - `Ok(true)` if the container exists and its status is `RUNNING`, `Ok(false)` otherwise (including if the container is stopped or does not exist).
///
/// # Errors
///
/// Returns `DevrsError::DockerApi` wrapped in `anyhow::Error` if inspecting the container fails for reasons other than "Not Found".
#[instrument(skip(name_or_id), fields(container = %name_or_id))] // Tracing span
pub async fn container_running(name_or_id: &str) -> Result<bool> {
    debug!("Checking running status for container: {}", name_or_id); // Log action

    // Attempt to inspect the container first.
    match inspect_container(name_or_id).await {
        // Inspection successful, container exists.
        Ok(details) => {
            // Check the state field within the details.
            let is_running = details
                .state // Option<ContainerState>
                .is_some_and(|s| {
                    // Check if the status enum is Some(RUNNING).
                    s.status == Some(ContainerStateStatusEnum::RUNNING)
                });
            debug!("Container '{}' running status: {}", name_or_id, is_running);
            Ok(is_running) // Return the boolean result.
        }
        // Inspection failed. Check if it was because the container wasn't found.
        Err(e)
            if e.downcast_ref::<DevrsError>().is_some_and(|err| {
                // Use downcast_ref for safe error type checking.
                matches!(err, DevrsError::ContainerNotFound { .. })
            }) =>
        {
            // Container doesn't exist, therefore it's not running.
            debug!("Container '{}' not found, thus not running.", name_or_id);
            Ok(false) // Return false gracefully.
        }
        // Inspection failed for some other reason.
        Err(e) => {
            error!(
                "Error checking running status for container '{}': {:?}",
                name_or_id, e
            );
            Err(e) // Propagate the underlying error.
        }
    }
}

/// Lists Docker containers, with options to include stopped containers and apply filters.
///
/// Wraps the `bollard` `list_containers` function, providing filtering capabilities
/// based on the Docker API standard filters (e.g., by label, status, name).
///
/// # Arguments
///
/// * `all` - If `true`, includes stopped and exited containers in the list. If `false`, only running containers are returned.
/// * `filters` - An optional `HashMap` where keys are Docker filter names (strings like "label", "status", "name")
///   and values are vectors of strings representing the filter criteria (e.g., `vec!["com.example.project=my-app"]`).
///
/// # Returns
///
/// * `Result<Vec<ContainerSummary>>` - A vector containing summary information for each container matching the criteria.
///
/// # Errors
///
/// Returns `DevrsError::DockerApi` wrapped in `anyhow::Error` if the Docker API call fails.
#[instrument(skip(all, filters), fields(all = %all, filters = ?filters))] // Tracing span
pub async fn list_containers(
    all: bool,
    filters: Option<HashMap<String, Vec<String>>>,
) -> Result<Vec<ContainerSummary>> {
    // Establish connection to Docker daemon.
    let docker = connect_docker().await?;
    // Prepare options for the list_containers API call.
    let options = Some(ListContainersOptions {
        all,                                  // Include all states or just running?
        filters: filters.unwrap_or_default(), // Use provided filters or empty map.
        ..Default::default()                  // Use defaults for other options (e.g., limit, size).
    });

    // Log the action being taken.
    info!(
        "Listing containers (All: {}, Filters: {:?})...",
        all,
        options.as_ref().map(|o| &o.filters) // Log filters if present.
    );

    // Call the bollard list_containers function and map potential errors.
    docker.list_containers(options).await.map_err(|e| {
        error!("Failed to list containers: {:?}", e);
        anyhow!(DevrsError::DockerApi { source: e }).context("Failed to list containers")
    })
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    // Note: Testing these functions effectively requires mocking the Docker API
    // responses provided by `bollard`. Unit tests would typically involve:
    // - Creating mock Docker clients.
    // - Defining expected responses for inspect_container and list_containers
    //   (including success cases, 404 errors, and other errors).
    // - Calling the functions in this module with the mock client.
    // - Asserting that the returned `Result<bool>`, `Result<ContainerInspectResponse>`,
    //   or `Result<Vec<ContainerSummary>>` matches the expected outcome based on the mock response.

    /// Placeholder test to ensure the module compiles.
    #[test]
    fn placeholder_state_test() {
        assert!(true);
    }

    // TODO: Implement mocked tests for container_exists, inspect_container,
    //       container_running, and list_containers.
}
