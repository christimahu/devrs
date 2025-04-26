//! # DevRS Error Types
//!
//! File: cli/src/core/error.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module defines the error types and error handling mechanisms used throughout
//! the DevRS application. It provides a consistent approach to error management
//! with detailed error information and context.
//!
//! ## Architecture
//!
//! The error system consists of two main components:
//! - `DevrsError`: A custom error enum using `thiserror` for specific error types
//! - `Result<T>`: A type alias for `anyhow::Result<T>` for flexible error handling
//!
//! The error types cover various domains:
//! - Configuration errors
//! - Filesystem errors
//! - Docker interaction errors
//! - Blueprint template errors
//! - Command execution errors
//!
//! ## Examples
//!
//! Using the error system:
//!
//! ```rust
//! // Return a specific error type
//! if !path.exists() {
//!     return Err(DevrsError::FileSystem(format!("Path not found: {}", path.display())))?;
//! }
//!
//! // Add context to errors using anyhow
//! let content = fs::read_to_string(&path)
//!     .with_context(|| format!("Failed to read file: {}", path.display()))?;
//!
//! // Pattern matching on error types
//! match result {
//!     Ok(value) => println!("Success: {}", value),
//!     Err(e) if e.downcast_ref::<DevrsError>().map_or(false, |de| matches!(de, DevrsError::ContainerNotFound { .. })) => {
//!         println!("Container not found, creating...");
//!     },
//!     Err(e) => return Err(e),
//! }
//! ```
//!
//! The error system provides detailed error messages to the user and
//! includes context information for debugging.
//!
use thiserror::Error;

/// Custom error type for the DevRS application.
// Removed PartialEq derive because source fields don't implement it.
#[derive(Error, Debug)]
pub enum DevrsError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Filesystem error: {0}")]
    FileSystem(String),

    #[error("Docker error: {0}")]
    Docker(String),

    #[error("Docker API interaction failed: {source}")]
    DockerApi {
        #[from]
        source: bollard::errors::Error,
    },

    #[error("Docker operation failed: {0}")]
    DockerOperation(String),

    #[error("Container '{name}' not found.")]
    ContainerNotFound { name: String },

    #[error("Image '{name}' not found.")]
    ImageNotFound { name: String },

    #[error("Container '{name}' is running. Stop it first or use --force.")]
    ContainerRunning { name: String },

    #[error("Image '{name}' is in use by one or more containers.")]
    ImageInUse { name: String },

    #[error("Template rendering error: {source}")]
    Template {
        #[from]
        source: tera::Error,
    },

    #[error("External command failed: {cmd}, Status: {status}, Output:\n{output}")]
    ExternalCommand {
        cmd: String,
        status: String,
        output: String,
    },

    #[error("Argument parsing error: {0}")]
    ArgumentParsing(String),
}

/// Type alias for Result using anyhow::Error for broad compatibility.
/// Anyhow allows for easy context addition and flexible error handling.
pub type Result<T> = anyhow::Result<T>;

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let config_err = DevrsError::Config("Missing setting 'foo'".to_string());
        assert_eq!(
            config_err.to_string(),
            "Configuration error: Missing setting 'foo'"
        );

        let container_not_found = DevrsError::ContainerNotFound {
            name: "test-container".into(),
        };
        assert_eq!(
            container_not_found.to_string(),
            "Container 'test-container' not found."
        );

        let image_in_use = DevrsError::ImageInUse {
            name: "test-image:latest".into(),
        };
        assert_eq!(
            image_in_use.to_string(),
            "Image 'test-image:latest' is in use by one or more containers."
        );
    }
}
