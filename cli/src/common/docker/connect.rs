//! # DevRS Docker Connection Helper
//!
//! File: cli/src/common/docker/connect.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!

//! ## Overview
//!
//! This internal utility module provides a single, standardized function,
//! `connect_docker`, responsible for establishing a connection to the local
//! Docker daemon using default settings provided by the `bollard` crate.
//! It centralizes connection logic and error handling for use by other
//! modules within `common::docker`.
//!
//! ## Architecture
//!
//! - Defines the asynchronous function `connect_docker`.
//! - Calls `bollard::Docker::connect_with_local_defaults()` to initiate the connection.
//! - Wraps potential connection errors from `bollard` into the application's
//!   standard `Result` type (`crate::core::error::Result`), mapping them to
//!   `DevrsError::DockerApi` and adding user-friendly context.
//!
//! ## Usage
//!
//! This function is intended for internal consumption by other modules in the
//! `common::docker` hierarchy.
//!
//! ```rust
//! // Example within another docker module (e.g., common/docker/images.rs)
//! use super::connect::connect_docker; // Import from sibling
//! use crate::core::error::Result;
//! use bollard::Docker;
//!
//! async fn perform_image_operation() -> Result<()> {
//!     // Get a connection to the Docker daemon.
//!     let docker: Docker = connect_docker().await?;
//!     // Use the 'docker' client instance...
//!     // docker.list_images(None).await?;
//!     Ok(())
//! }
//! ```
//!
use crate::core::error::{DevrsError, Result}; // Use Result from core::error
use anyhow::{anyhow, Context}; // For error context
use bollard::Docker; // Docker client struct
use tracing::instrument; // For tracing function calls

/// Establishes a connection to the local Docker daemon using default settings.
///
/// This function attempts to connect to the Docker daemon typically found at
/// standard locations (e.g., `/var/run/docker.sock` on Unix, named pipe on Windows)
/// using `bollard::Docker::connect_with_local_defaults`.
///
/// # Returns
///
/// * `Result<Docker>` - A `bollard::Docker` client instance wrapped in a `Result`
///   on successful connection.
///
/// # Errors
///
/// Returns an `Err` wrapping `DevrsError::DockerApi` if the connection fails.
/// The error includes context suggesting potential causes like the Docker daemon
/// not running or being inaccessible.
#[instrument] // Automatically adds tracing spans for function entry/exit.
pub async fn connect_docker() -> Result<Docker> {
    Docker::connect_with_local_defaults()
        // Map the bollard::Error into our application's error handling structure.
        .map_err(|e| anyhow!(DevrsError::DockerApi { source: e }))
        // Add user-friendly context to the error if connection fails.
        .context("Failed to connect to Docker daemon. Is it running and accessible?")
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*; // Import items from the parent module (connect.rs).

    /// Test successful connection to a running Docker daemon.
    /// This test is marked `#[ignore]` because it requires an external dependency
    /// (a running and accessible Docker daemon) which may not be present in all
    /// testing environments (like CI). Run locally with `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore] // Ignored because it requires a running Docker daemon.
    async fn test_connect_docker_success() {
        // This test assumes a Docker daemon is running and accessible.
        let result = connect_docker().await;
        // Assert that the connection attempt resulted in Ok.
        assert!(
            result.is_ok(),
            "Should connect successfully if Docker is running"
        );
    }

    // Future test idea: Add a test case for failure if Docker connection
    // can be reliably simulated as unavailable (e.g., by temporarily stopping
    // the Docker service, though this is complex for automated tests).
}
