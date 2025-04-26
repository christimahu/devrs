//! # DevRS Filesystem Copy Operations
//!
//! File: cli/src/common/fs/copy.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!

//! ## Overview
//!
//! This module provides utilities for copying files and directories within the
//! filesystem. Currently, it focuses on recursive directory copying, a common
//! requirement for tasks like scaffolding new projects from blueprint templates.
//!
//! ## Architecture
//!
//! The primary function, `copy_directory_recursive`, utilizes the external `fs_extra`
//! crate to handle the complexities of recursive copying. This includes:
//! - Creating the target directory structure if it doesn't exist.
//! - Overwriting existing files within the target directory by default.
//! - Attempting to preserve file attributes where possible (though this depends on `fs_extra`'s behavior).
//!
//! Error handling wraps `fs_extra` errors into the application's standard `Result` type using `anyhow` for context.
//!
//! **Note:** There's an intention mentioned in comments to potentially replace `fs_extra`
//! with a custom implementation later for finer-grained control.
//!
//! ## Usage
//!
//! This utility is mainly used internally by commands like `devrs blueprint create`.
//!
//! ```rust
//! use crate::common::fs::copy; // Assuming re-export via common::fs
//! use crate::core::error::Result;
//! use std::path::Path;
//!
//! # fn run_example() -> Result<()> {
//! let source_template_path = Path::new("./blueprints/rust");
//! let target_project_path = Path::new("./my_new_rust_project");
//!
//! // Recursively copy the template contents to the new project directory.
//! copy::copy_directory_recursive(source_template_path, target_project_path)?;
//! println!("Blueprint copied successfully!");
//! # Ok(())
//! # }
//! ```
//!
use crate::core::error::Result; // Use standard Result type from core::error
use std::path::Path; // Filesystem path type
use tracing::{info, warn}; // Logging utilities

/// Copies a directory recursively from a source path to a target path.
///
/// This function leverages the `fs_extra` crate to perform the copy operation.
/// It handles creating necessary parent directories for the target and will
/// overwrite existing files in the target directory by default.
///
/// **Warning:** Currently uses `fs_extra` as noted by the warning log.
/// Future versions might use a custom implementation.
///
/// # Arguments
///
/// * `source` - A `&Path` reference to the source directory to copy from. Must exist.
/// * `target` - A `&Path` reference to the target directory or path. If it exists,
///   its contents may be overwritten. Parent directories will be created if needed.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` on successful completion of the copy operation.
///
/// # Errors
///
/// Returns an `Err` if:
/// - The source path does not exist or is not accessible.
/// - The target path cannot be created or written to.
/// - Any error occurs during the file copying process (e.g., I/O errors, permission issues),
///   wrapped with context by `anyhow`.
#[allow(dead_code)] // Allow function to be unused if calling code (e.g., blueprint create) isn't fully implemented yet.
pub fn copy_directory_recursive(source: &Path, target: &Path) -> Result<()> {
    // Log a warning indicating the underlying library being used.
    warn!(
        "Using fs_extra for recursive copy: fs::copy::copy_directory_recursive (Source: {:?}, Target: {:?})",
        source, target
    );
    // Log the start of the operation.
    info!("Starting recursive copy from {:?} to {:?}", source, target);

    // --- Implementation using fs_extra ---
    // Configure copy options provided by the `fs_extra` crate.
    let mut options = fs_extra::dir::CopyOptions::new();
    // Allow overwriting files in the target directory if they already exist.
    options.overwrite = true;
    // Add more options here if needed (e.g., skip_exist, buffer_size, copy_inside).

    // Perform the recursive directory copy using `fs_extra`.
    fs_extra::dir::copy(source, target, &options)
        // Map the `fs_extra::error::Error` into the application's standard `Result`.
        .map_err(|e| {
            // Convert the fs_extra error into an anyhow::Error for flexible handling.
            anyhow::anyhow!(e)
                // Add context indicating the operation that failed.
                .context(format!("Failed to copy dir {:?} to {:?}", source, target))
        })?; // Propagate the error if the copy fails.

    // Log the successful completion of the operation.
    info!("Finished recursive copy from {:?} to {:?}", source, target);
    Ok(()) // Return Ok on success.
}

// --- Unit Tests ---
// Test the recursive copy logic using temporary directories.
#[cfg(test)]
mod tests {
    // TODO: Add more tests:
    // - Test overwriting existing files/directories in the target.
    // - Test copying empty directories.
    // - Test error handling (e.g., source doesn't exist, permission errors during copy).
    // - Test handling of symbolic links (if relevant, depends on fs_extra options/behavior).
}
