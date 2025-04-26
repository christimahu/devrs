//! # DevRS Docker Image Operations
//!
//! File: cli/src/common/docker/images.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!

//! ## Overview
//!
//! This module provides functions specifically for interacting with Docker images
//! stored locally on the Docker host. It leverages the `bollard` crate to offer
//! operations like checking for image existence, inspecting image details, listing
//! available images, and removing images.
//!
//! ## Architecture
//!
//! Key functions implemented:
//! - **`image_exists`**: Performs a quick check using `inspect_image` to see if an image tag/ID is present locally.
//! - **`inspect_image`**: Retrieves detailed metadata about a specific image (layers, config, etc.).
//! - **`list_images`**: Fetches a list of local images, optionally including intermediate layers or applying filters.
//! - **`remove_image`**: Attempts to remove one or more specified images from the local cache.
//!
//! All functions handle communication with the Docker daemon via the `connect_docker` helper
//! and map potential Docker API errors (e.g., image not found, image in use) to
//! appropriate `DevrsError` variants for consistent error handling across the application.
//!
//! ## Usage
//!
//! These functions are typically called by command handlers (like `devrs container build`,
//! `devrs container rmi`, `devrs env status`) often via re-exports from `common::docker::mod`.
//!
//! ```rust
//! use crate::common::docker::images; // Direct import (or use re-export)
//! use crate::core::error::Result;
//! use std::collections::HashMap;
//!
//! # async fn run_example() -> Result<()> {
//! # let image_name = "my-app:latest";
//! // Check if the image exists
//! if images::image_exists(image_name).await? {
//!     // Inspect the image if needed
//!     let details = images::inspect_image(image_name).await?;
//!     println!("Image ID: {}", details.id.unwrap_or_default());
//!
//!     // Attempt to remove the image (force=false)
//!     match images::remove_image(image_name, false).await {
//!         Ok(()) => println!("Image removed."),
//!         Err(e) => println!("Failed to remove image: {}", e),
//!     }
//! } else {
//!     println!("Image does not exist locally.");
//! }
//!
//! // List all images tagged 'latest'
//! let mut filters = HashMap::new();
//! filters.insert("reference".to_string(), vec!["*:latest".to_string()]);
//! let latest_images = images::list_images(false, Some(filters)).await?;
//! for img in latest_images {
//!     println!("Found latest image: {:?}", img.repo_tags);
//! }
//! # Ok(())
//! # }
//! ```
//!
use crate::core::error::{DevrsError, Result}; // Use standard Result and custom Error
use anyhow::anyhow; // For error context wrapping
use bollard::{
    image::{ListImagesOptions, RemoveImageOptions}, // Specific options structs for image operations
    models::ImageInspect,                           // Response struct for inspect_image
    models::ImageSummary,                           // Response struct element for list_images
};
use std::collections::HashMap; // For list_images filters
use tracing::{debug, error, info, instrument, warn}; // Logging utilities

// Use the shared connection helper from the sibling module.
use super::connect::connect_docker;

/// Inspects a Docker image by name or ID to retrieve detailed metadata.
///
/// This function provides comprehensive information about an image, including its layers,
/// configuration (like environment variables, entrypoint), creation date, etc.
/// For a simple existence check, prefer using `image_exists`.
///
/// # Arguments
///
/// * `name_or_id` - The name (e.g., "ubuntu:latest") or ID (full or prefix) of the image to inspect.
///
/// # Returns
///
/// * `Result<ImageInspect>` - A struct containing the detailed image information on success.
///
/// # Errors
///
/// * `DevrsError::ImageNotFound` - If no image matching the provided name or ID exists locally (maps Docker 404).
/// * `DevrsError::DockerApi` - For other errors during communication with the Docker daemon.
#[instrument(skip(name_or_id), fields(image = %name_or_id))] // Tracing span
#[allow(dead_code)] // Allow function to be unused for now if needed
pub async fn inspect_image(name_or_id: &str) -> Result<ImageInspect> {
    // Establish connection to Docker daemon.
    let docker = connect_docker().await?;
    debug!("Inspecting image: {}", name_or_id); // Log action

    // Call the bollard inspect_image function.
    docker
        .inspect_image(name_or_id)
        .await
        // Map potential errors to our custom error types.
        .map_err(|e| match e {
            // Handle the specific case where the image is not found (404).
            bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            } => {
                warn!(
                    "Inspection failed because image '{}' was not found.",
                    name_or_id
                );
                // Create our specific ImageNotFound error.
                anyhow!(DevrsError::ImageNotFound {
                    name: name_or_id.to_string()
                })
            }
            // Handle all other Docker API errors generically.
            _ => {
                error!("Failed to inspect image '{}': {:?}", name_or_id, e);
                // Wrap the original bollard error in our DockerApi error.
                anyhow!(DevrsError::DockerApi { source: e })
                    .context(format!("Failed to inspect image '{}'", name_or_id))
            }
        })
}

/// Checks if a Docker image exists locally by name or ID.
///
/// This is a convenience function that uses `inspect_image` internally
/// but only returns a boolean, simplifying checks for image presence.
///
/// # Arguments
///
/// * `name_or_id` - The name (e.g., "ubuntu:latest") or ID (full or prefix) of the image to check.
///
/// # Returns
///
/// * `Result<bool>` - `Ok(true)` if the image exists locally, `Ok(false)` if it does not.
///
/// # Errors
///
/// * `DevrsError::DockerApi` - For errors during communication with the Docker daemon (other than 404 Not Found).
#[instrument(skip(name_or_id), fields(image = %name_or_id))] // Tracing span
pub async fn image_exists(name_or_id: &str) -> Result<bool> {
    // Establish connection to Docker daemon.
    let docker = connect_docker().await?;
    debug!("Checking existence of image: {}", name_or_id); // Log action

    // Attempt to inspect the image.
    match docker.inspect_image(name_or_id).await {
        // If inspection succeeds, the image exists.
        Ok(_) => {
            debug!("Image '{}' found locally.", name_or_id);
            Ok(true)
        }
        // If inspection returns a 404 error, the image does not exist.
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {
            debug!("Image '{}' not found locally.", name_or_id);
            Ok(false)
        }
        // For any other error during inspection, propagate it.
        Err(e) => {
            error!(
                "Error during existence check for image '{}': {:?}",
                name_or_id, e
            );
            Err(
                anyhow!(DevrsError::DockerApi { source: e }).context(format!(
                    "Failed to check existence for image '{}'",
                    name_or_id
                )),
            )
        }
    }
}

/// Lists Docker images available locally on the Docker host.
///
/// Allows filtering based on various criteria supported by the Docker API
/// (e.g., `dangling=true`, `label=key=value`). Also supports listing
/// intermediate image layers if `all` is set to true.
///
/// # Arguments
///
/// * `all` - If `true`, includes intermediate image layers in the results. If `false`, only shows top-level images.
/// * `filters` - An optional `HashMap` specifying Docker API filters. Keys are filter names (e.g., "dangling", "label", "reference"),
///   and values are vectors of strings representing the filter values.
///
/// # Returns
///
/// * `Result<Vec<ImageSummary>>` - A vector containing summary information for each image matching the criteria.
///
/// # Errors
///
/// * `DevrsError::DockerApi` - For errors during communication with the Docker daemon.
#[instrument(skip(all, filters))] // Tracing span, skipping potentially large filters map
pub async fn list_images(
    all: bool,
    filters: Option<HashMap<String, Vec<String>>>,
) -> Result<Vec<ImageSummary>> {
    // Establish connection to Docker daemon.
    let docker = connect_docker().await?;
    // Prepare options for the list_images API call.
    let options = Some(ListImagesOptions {
        all,                                  // Include intermediate layers?
        filters: filters.unwrap_or_default(), // Use provided filters or an empty map.
        ..Default::default()                  // Use defaults for other options (e.g., digests).
    });

    // Log the action being taken.
    info!(
        "Listing images (All: {}, Filters: {:?})...",
        all,
        options.as_ref().map(|o| &o.filters) // Log filters for debugging if present.
    );

    // Call the bollard list_images function and map potential errors.
    docker
        .list_images(options)
        .await
        .map_err(|e| anyhow!(DevrsError::DockerApi { source: e }).context("Failed to list images"))
}

/// Removes a specified Docker image from the local cache.
///
/// Allows forcing removal, which may be necessary if the image is tagged multiple times
/// or used by stopped containers.
///
/// # Arguments
///
/// * `name_or_id` - The name (e.g., "ubuntu:latest") or ID (full or prefix) of the image to remove.
/// * `force` - If `true`, attempts to force the removal. Note: This typically does *not* remove images used by *running* containers.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` on successful removal.
///
/// # Errors
///
/// * `DevrsError::ImageNotFound` - If the specified image does not exist locally (maps Docker 404).
/// * `DevrsError::ImageInUse` - If the image cannot be removed because it's currently in use by a container (maps Docker 409 Conflict).
/// * `DevrsError::DockerApi` - For other errors during communication with the Docker daemon.
#[instrument(skip(name_or_id, force), fields(image = %name_or_id))] // Tracing span
pub async fn remove_image(name_or_id: &str, force: bool) -> Result<()> {
    // Establish connection to Docker daemon.
    let docker = connect_docker().await?;
    info!("Removing image '{}' (Force: {})...", name_or_id, force); // Log action.
                                                                    // Prepare options for the remove_image API call.
    let options = Some(RemoveImageOptions {
        force,          // Force removal?
        noprune: false, // Set true to *prevent* removal of untagged parent layers. Default false is usually desired.
    });

    // Call the bollard remove_image function.
    match docker.remove_image(name_or_id, options, None).await {
        // Removal successful, Docker API returns a list of actions performed (deleted layers, untagged references).
        Ok(results) => {
            // Log the details returned by Docker for debugging/information.
            for result in results {
                if let Some(deleted) = result.deleted {
                    info!("Deleted: {}", deleted);
                }
                if let Some(untagged) = result.untagged {
                    info!("Untagged: {}", untagged);
                }
            }
            info!("Image '{}' removed successfully.", name_or_id);
            Ok(()) // Indicate overall success.
        }
        // Handle specific error status codes returned by Docker.
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, .. // Image not found.
        }) => {
            warn!("Image '{}' not found, cannot remove.", name_or_id);
            Err(anyhow!(DevrsError::ImageNotFound {
                name: name_or_id.to_string()
            }))
        }
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 409, // Conflict error (image likely in use).
            message,          // Docker usually provides a reason in the message.
        }) => {
            error!("Conflict removing image '{}': {}", name_or_id, message);
            // Return our specific ImageInUse error with context.
            Err(anyhow!(DevrsError::ImageInUse { name: name_or_id.to_string() })
                .context(format!("Image '{}' is in use by a container. Stop and remove the container first, or use --force (for stopped containers).", name_or_id)))
        }
        // Handle any other Docker API errors.
        Err(e) => {
             error!("Failed to remove image '{}': {:?}", name_or_id, e);
             Err(anyhow!(DevrsError::DockerApi { source: e })
                 .context(format!("Failed to remove image '{}'", name_or_id)))
        }
    }
}
