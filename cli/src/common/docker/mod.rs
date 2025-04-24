//! # DevRS Docker Module Interface
//!
//! File: cli/src/common/docker/mod.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!

//! ## Overview
//!
//! This module serves as the central public interface for interacting with Docker
//! within the DevRS CLI. It organizes Docker-related functionality into logical
//! submodules and re-exports commonly used functions for convenience, abstracting
//! the underlying `bollard` crate interactions.
//!
//! ## Architecture
//!
//! The `common::docker` module delegates tasks to the following specialized submodules:
//!
//! - **`connect`**: Handles establishing the connection to the Docker daemon.
//! - **`images`**: Manages Docker images (checking existence, listing, inspection, removal).
//! - **`state`**: Queries the status and details of Docker containers (existence, running state, inspection, listing).
//! - **`lifecycle`**: Controls the lifecycle of containers (starting, stopping, removing, ensuring the core environment is running).
//! - **`operations`**: Implements core actions like building images (`build_image`) and creating/starting containers (`run_container`).
//! - **`interaction`**: Facilitates interaction with running containers, such as executing commands (`exec_in_container`) and streaming logs (`get_container_logs`).
//!
//! By re-exporting key functions, this module provides a simplified API surface for
//! other parts of the application that need to perform Docker operations.
//!
//! ## Usage
//!
//! Command handlers and other utilities interact with Docker primarily through this module's
//! re-exported functions:
//!
//! ```rust
//! use crate::common::docker; // Import the main docker module interface
//! use crate::core::error::Result;
//! # use crate::core::config;
//! # use std::collections::HashMap;
//!
//! # async fn run_example() -> Result<()> {
//! # let image_name = "my-image:latest";
//! # let container_name = "my-container";
//! # let cfg = config::Config::default();
//!
//! // Build an image (uses re-exported build_image from operations)
//! // docker::build_image(image_name, "Dockerfile", ".", false).await?;
//!
//! // Check if a container is running (uses re-exported container_running from state)
//! let is_running = docker::container_running(container_name).await?;
//! if is_running {
//!     // Stop the container (uses re-exported stop_container from lifecycle)
//!     docker::stop_container(container_name, Some(5)).await?;
//! }
//! # Ok(())
//! # }
//! ```
//!

/// Handles establishing a connection to the local Docker daemon.
pub mod connect;
/// Provides operations specific to Docker images (existence checks, listing, removal).
pub mod images;
/// Facilitates interaction with running containers (executing commands, retrieving logs).
pub mod interaction;
/// Contains functions for managing the lifecycle of containers (start, stop, remove).
pub mod lifecycle;
/// Implements core Docker actions like building images and running containers.
pub mod operations;
/// Offers functions to query the state of containers (existence, running status, inspection).
pub mod state;

// --- Re-exports for easier access from other parts of the application ---
// Makes functions available like `docker::build_image(...)` instead of `docker::operations::build_image(...)`

// Core Operations (Build/Run - Reside in operations.rs)
pub use operations::build_image;
pub use operations::run_container;

// Image Operations (from images.rs)
pub use images::image_exists;

// --- Unit Tests (Module Level) ---
#[cfg(test)]
mod tests {
    // This test simply ensures the module itself compiles.
    // More specific tests reside within each submodule (`connect`, `images`, etc.).
    #[test]
    fn placeholder_docker_mod_test() {
        // This test doesn't do much, just ensures the file compiles.
        // More meaningful tests are within the submodules.
        assert!(true);
    }
}
