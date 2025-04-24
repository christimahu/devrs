//! # DevRS Filesystem I/O Operations
//!
//! File: cli/src/common/fs/io.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!

//! ## Overview
//!
//! This module centralizes fundamental filesystem input/output (I/O) operations
//! required by various parts of the DevRS application. It provides convenient,
//! robust wrappers around standard library `std::fs` functions for tasks such as
//! ensuring directories exist, reading entire files into strings, and writing
//! string content back to files.
//!
//! ## Architecture
//!
//! The module offers several focused utility functions:
//! - **`ensure_dir_exists`**: Checks if a directory exists at the given path. If not, it creates the directory, including any necessary parent directories (`fs::create_dir_all`). It also validates that if a path *does* exist, it is actually a directory.
//! - **`read_file_to_string`**: A simple wrapper around `fs::read_to_string` that adds context to potential I/O errors using `anyhow::Context`.
//! - **`write_string_to_file`**: Writes a string slice (`&str`) to the specified file path. Before writing, it ensures the parent directory of the target file exists by calling `ensure_dir_exists`. It overwrites the file if it already exists. Errors during directory creation or writing are wrapped with context.
//!
//! These functions aim to simplify common I/O patterns and provide consistent error handling with helpful context messages.
//!
//! ## Usage
//!
//! These utilities are broadly used, for example:
//! - `setup` commands use `ensure_dir_exists` and `write_string_to_file` for configuration files.
//! - `blueprint create` uses `ensure_dir_exists` and `write_string_to_file` when rendering template files.
//! - `config` loading might use `read_file_to_string`.
//!
//! ```rust
//! use crate::common::fs::io; // Assuming re-export via common::fs
//! use crate::core::error::Result;
//! use std::path::Path;
//!
//! # fn run_example() -> Result<()> {
//! let log_dir = Path::new("./logs/app_logs");
//! let config_file = Path::new("./config/settings.toml");
//! let output_file = Path::new("./output/data.txt");
//!
//! // Ensure a directory exists, creating intermediates if needed.
//! io::ensure_dir_exists(log_dir)?;
//!
//! // Read a configuration file.
//! let config_content = io::read_file_to_string(config_file)?;
//!
//! // Write some processed data to an output file.
//! let data_to_write = "Processed data results.";
//! io::write_string_to_file(output_file, data_to_write)?;
//! # Ok(())
//! # }
//! ```
//!
use crate::core::error::{DevrsError, Result}; // Use standard Result and custom Error types
use anyhow::Context; // For adding context to errors
use std::fs; // Standard filesystem module
use std::path::Path; // Filesystem path type
use tracing::{debug, info}; // Logging utilities

/// Ensures that a directory exists at the specified path.
///
/// If the path does not exist, this function attempts to create the directory,
/// including any necessary parent directories (similar to `mkdir -p`).
/// If the path already exists but is not a directory (e.g., it's a file),
/// an error (`DevrsError::FileSystem`) is returned.
///
/// # Arguments
///
/// * `path` - A `&Path` reference to the directory path to ensure exists.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if the directory exists or was successfully created.
///
/// # Errors
///
/// Returns an `Err` if:
/// - The path exists but is not a directory.
/// - Creating the directory fails (e.g., due to permissions).
#[allow(dead_code)] // Allow function to be unused if not all callers are implemented yet.
pub fn ensure_dir_exists(path: &Path) -> Result<()> {
    // Check if the path exists in the filesystem.
    if !path.exists() {
        // Path does not exist, attempt to create it recursively.
        fs::create_dir_all(path)
            // Add context to any error occurring during directory creation.
            .with_context(|| format!("Failed to create directory {:?}", path))?;
        // Log the successful creation.
        info!("Created directory: {:?}", path);
    }
    // Path exists, check if it's actually a directory.
    else if !path.is_dir() {
        // It exists but is not a directory (e.g., a file). Return an error.
        // Use anyhow::bail! for a concise error return, wrapping our custom error type.
        anyhow::bail!(DevrsError::FileSystem(format!(
            "Path exists but is not a directory: {:?}",
            path
        )));
    }
    // Path exists and is already a directory.
    else {
        // Log that no action was needed (debug level).
        debug!("Directory already exists: {:?}", path);
    }
    // If we reach here, the directory exists (either pre-existing or newly created).
    Ok(())
}

/// Reads the entire content of a file into a string.
///
/// This is a simple wrapper around `std::fs::read_to_string` that adds
/// contextual information to the error message if reading fails.
///
/// # Arguments
///
/// * `path` - A `&Path` reference to the file to read.
///
/// # Returns
///
/// * `Result<String>` - Returns `Ok(String)` containing the file content if successful.
///
/// # Errors
///
/// Returns an `Err` if the file cannot be found, opened, or read (e.g., permissions, I/O error),
/// with context indicating which file failed.
#[allow(dead_code)] // Allow function to be unused if not all callers are implemented yet.
pub fn read_file_to_string(path: &Path) -> Result<String> {
    // Call the standard library function and enhance any error with context.
    fs::read_to_string(path).with_context(|| format!("Failed to read file {:?}", path))
}

/// Writes string content to a specified file path, overwriting if it exists.
///
/// This function first ensures that the parent directory of the target `path` exists,
/// creating it recursively if necessary using `ensure_dir_exists`. It then writes
/// the provided `content` string slice to the file. If the file already exists,
/// its contents will be replaced.
///
/// # Arguments
///
/// * `path` - A `&Path` reference to the target file path.
/// * `content` - A `&str` slice containing the content to write to the file.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if the file was successfully written.
///
/// # Errors
///
/// Returns an `Err` if:
/// - The parent directory cannot be created.
/// - Writing to the file fails (e.g., permissions, I/O error).
#[allow(dead_code)] // Allow function to be unused if not all callers are implemented yet.
pub fn write_string_to_file(path: &Path, content: &str) -> Result<()> {
    // --- Ensure Parent Directory Exists ---
    // Get the parent directory of the target file path.
    if let Some(parent) = path.parent() {
        // If a parent exists, ensure it's a directory (creates if needed).
        ensure_dir_exists(parent)?; // Propagate error if directory creation fails.
    }
    // If `path.parent()` returns None, it means the path is likely a root path
    // (e.g., "/" or "C:\"), which should already exist.

    // --- Write File Content ---
    // Attempt to write the string content to the file.
    fs::write(path, content)
        // Add context to any error during writing.
        .with_context(|| format!("Failed to write to file {:?}", path))?;
    // Log the successful write operation.
    info!("Wrote content to file: {:?}", path);
    Ok(()) // Indicate success.
}

// --- Unit Tests ---
// Tests for the filesystem I/O utilities.
#[cfg(test)]
mod tests {
    use super::*; // Import items from the parent module (io.rs).
    use tempfile::tempdir; // Create temporary directories for isolated testing.

    /// Test `ensure_dir_exists` when the directory needs to be created, including parents.
    #[test]
    fn test_ensure_dir_exists_creates_new() -> Result<()> {
        // Setup: Create a temporary base directory.
        let base_dir = tempdir()?;
        // Define a path for a new directory structure *within* the base directory.
        let new_dir = base_dir.path().join("new/subdir");
        // Assert: Ensure the target directory does not exist initially.
        assert!(!new_dir.exists());
        // Action: Call the function to ensure the directory exists.
        ensure_dir_exists(&new_dir)?;
        // Assert: Verify the directory now exists and is actually a directory.
        assert!(new_dir.is_dir());
        Ok(()) // Test passes.
    }

    /// Test `ensure_dir_exists` when the directory already exists.
    #[test]
    fn test_ensure_dir_exists_already_exists() -> Result<()> {
        // Setup: Create a temporary base directory.
        let base_dir = tempdir()?;
        // Define a path within the base directory.
        let existing_dir = base_dir.path().join("existing");
        // Manually create the directory beforehand.
        fs::create_dir(&existing_dir)?;
        // Action: Call the function on the existing directory.
        ensure_dir_exists(&existing_dir)?; // Should be a no-op and succeed.
        // Assert: Verify the directory still exists and is a directory.
        assert!(existing_dir.is_dir());
        Ok(()) // Test passes.
    }

    /// Test `ensure_dir_exists` when the target path exists but is a file.
    #[test]
    fn test_ensure_dir_exists_path_is_file() -> Result<()> {
        // Setup: Create a temporary base directory.
        let base_dir = tempdir()?;
        // Define a path and create a *file* at that path.
        let file_path = base_dir.path().join("a_file.txt");
        fs::write(&file_path, "hello")?;
        // Action: Call the function trying to ensure this path is a directory.
        let result = ensure_dir_exists(&file_path);
        // Assert: Expect an error because the path exists but is not a directory.
        assert!(result.is_err());
        // Assert: Check the error message content for correctness.
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Path exists but is not a directory"));
        Ok(()) // Test passes (error was expected).
    }

    /// Test both writing to and reading from a file using the utility functions.
    #[test]
    fn test_read_write_string_to_file() -> Result<()> {
        // Setup: Create a temporary base directory.
        let base_dir = tempdir()?;
        // Define the path for the test file.
        let file_path = base_dir.path().join("test_rw.txt");
        let content = "Hello, DevRS!"; // Content to write.
        // Action: Write the string content to the file.
        write_string_to_file(&file_path, content)?;
        // Assert: Verify the file was created.
        assert!(file_path.exists());
        // Action: Read the content back from the file.
        let read_content = read_file_to_string(&file_path)?;
        // Assert: Verify the content read matches the content written.
        assert_eq!(read_content, content);
        Ok(()) // Test passes.
    }

    /// Test `read_file_to_string` when the target file does not exist.
    #[test]
    fn test_read_file_not_found() -> Result<()> {
        // Setup: Create a temporary base directory.
        let base_dir = tempdir()?;
        // Define a path to a file that does not exist.
        let file_path = base_dir.path().join("nonexistent.txt");
        // Action: Attempt to read the non-existent file.
        let result = read_file_to_string(&file_path);
        // Assert: Expect an error because the file was not found.
        assert!(result.is_err());
        Ok(()) // Test passes (error was expected).
    }
}
