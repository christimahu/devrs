//! # DevRS Filesystem Link Operations
//!
//! File: cli/src/common/fs/links.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!

//! ## Overview
//!
//! This module provides cross-platform utilities for creating symbolic links (symlinks)
//! within the filesystem. Symbolic links are used, for example, by the `devrs setup`
//! commands to link configuration files (like Neovim's `init.lua`) from the DevRS
//! configuration directory to their expected locations in the user's home directory.
//!
//! ## Architecture
//!
//! The primary function is `create_symlink`. Its logic includes:
//! - **Source Validation:** Ensures the `source` path (the file/directory the link should point to) actually exists.
//! - **Target Parent Creation:** Ensures the parent directory of the `target` path (where the link will be created) exists, creating it if necessary using `io::ensure_dir_exists`.
//! - **Existing Target Handling:**
//!     - Checks if the `target` path already exists (either as a file, directory, or existing link).
//!     - If it's an existing symlink, it checks if it already points to the correct canonicalized `source` path. If so, the operation is skipped (idempotent).
//!     - If the target exists and is *not* the correct link, it is backed up by renaming it (appending `.devrs_backup`).
//! - **Platform-Specific Link Creation:** Uses `std::os::unix::fs::symlink` on Unix-like systems and `std::os::windows::fs::symlink_dir` or `symlink_file` on Windows to create the appropriate type of link.
//! - **Error Handling:** Uses `anyhow` to provide context to filesystem errors.
//!
//! ## Usage
//!
//! This utility is mainly intended for internal use by setup scripts or commands.
//!
//! ```rust
//! use crate::common::fs::links; // Assuming re-export via common::fs
//! use crate::core::error::Result;
//! use std::path::Path;
//!
//! # fn run_example() -> Result<()> {
//! // Define the source file (e.g., in ~/.config/devrs/config)
//! let source_config = Path::new("/path/to/devrs/config/nvim/init.lua");
//! // Define the target location where the link should be created
//! let target_link_location = Path::new("/home/user/.config/nvim/init.lua");
//!
//! // Attempt to create the symlink. This handles backup, parent dirs, etc.
//! links::create_symlink(source_config, target_link_location)?;
//! println!("Symlink created or verified successfully.");
//! # Ok(())
//! # }
//! ```
//!
use crate::common::fs::io::ensure_dir_exists; // Use helper from sibling io module
use crate::core::error::Result; // Use standard Result type from core::error
use anyhow::{Context, bail}; // For context and concise error returns
use std::path::Path; // Filesystem path type
use tracing::{debug, info, warn}; // Logging utilities

/// Creates a symbolic link from a `source` path to a `target` path.
///
/// This function handles several common scenarios:
/// - Ensures the `source` path exists.
/// - Ensures the parent directory of the `target` path exists, creating it if needed.
/// - If the `target` path already exists:
///     - If it's already a symlink pointing to the correct canonicalized `source`,
///       the function does nothing and returns `Ok`.
///     - Otherwise, the existing item at `target` is renamed to `<target_name>.devrs_backup`
///       before the new symlink is created.
/// - Uses platform-specific functions (`std::os::unix::fs::symlink` or `std::os::windows::fs::symlink_file`/`symlink_dir`)
///   to create the link.
///
/// # Arguments
///
/// * `source` - A `&Path` reference to the existing file or directory that the link should point to.
/// * `target` - A `&Path` reference to the desired location where the symbolic link should be created.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if the symlink was created successfully or already existed correctly.
///
/// # Errors
///
/// Returns an `Err` if:
/// - The `source` path does not exist.
/// - The parent directory of the `target` cannot be created.
/// - Renaming an existing item at the `target` path fails.
/// - Creating the symbolic link itself fails (e.g., permissions, unsupported filesystem).
/// - The platform is not Unix or Windows (as automatic symlink creation is only implemented for these).
#[allow(dead_code)] // Allow function to be unused if not all callers are implemented yet.
pub fn create_symlink(source: &Path, target: &Path) -> Result<()> {
    // Log the requested operation.
    info!("Creating symlink from {:?} to {:?}", source, target);

    // 1. Ensure the source path actually exists.
    if !source.exists() {
        // Use bail! for a concise error return.
        bail!("Symlink source path does not exist: {:?}", source);
    }

    // 2. Ensure the parent directory of the target link exists.
    if let Some(parent) = target.parent() {
        ensure_dir_exists(parent) // Use the helper from the io module.
            .with_context(|| {
                // Add context specific to this operation.
                format!("Failed to create parent directory for target {:?}", target)
            })?; // Propagate error if parent creation fails.
    }

    // 3. Handle existing item at the target path.
    // Check if path exists using `symlink_metadata` first, which doesn't follow links.
    if target.symlink_metadata().is_ok() {
        // Target path exists (could be file, dir, or existing link).
        // Check specifically if it's already the *correct* symlink.
        if let Ok(existing_link_target_path) = std::fs::read_link(target) {
            // Successfully read the target of the existing link.
            debug!(
                "Target {:?} exists and is a symlink pointing to {:?}",
                target, existing_link_target_path
            );

            // Resolve paths to their canonical forms to handle relative paths, '.', '..'.
            // Use unwrap_or_else to handle potential errors during canonicalization
            // (e.g., if source doesn't exist, though we checked earlier). Fallback to original paths.
            let canonical_source = source
                .canonicalize()
                .unwrap_or_else(|_| source.to_path_buf());

            // To resolve the existing link's target relative to the link's location:
            // Get the parent dir of the link itself.
            let target_parent = target.parent().unwrap_or_else(|| Path::new("."));
            // Join the parent dir with the potentially relative path read from the link.
            let canonical_existing_target = target_parent
                .join(&existing_link_target_path)
                .canonicalize() // Canonicalize the resolved path.
                // Fallback to the original link target path if canonicalization fails.
                .unwrap_or(existing_link_target_path);

            debug!(
                "Canonical Source: {:?}, Canonical Existing Target: {:?}",
                canonical_source, canonical_existing_target
            );

            // Compare the canonical paths. If they match, the correct link exists.
            if canonical_source == canonical_existing_target {
                debug!("Symlink already exists and is correct: {:?}", target);
                return Ok(()); // Nothing more to do.
            }
            // If we reach here, it's a symlink but points elsewhere. It needs backup.
        }

        // If it exists but isn't the correct symlink (or isn't a symlink at all), back it up.
        // Construct a backup path name (e.g., target.link -> target.link.devrs_backup).
        let backup_path = target.with_extension(format!(
            // Append .devrs_backup, preserving original extension if possible.
            "{}.devrs_backup",
            target
                .extension() // Get original extension (Option<&OsStr>)
                .unwrap_or_default() // Use empty OsStr if no extension
                .to_str() // Convert OsStr to Option<&str>
                .unwrap_or("") // Use empty string if conversion fails
        ));
        warn!( // Log the backup operation clearly.
            "Target {:?} exists or is a different link, backing up to {:?}",
            target, backup_path
        );
        // Attempt to rename the existing item to the backup path.
        std::fs::rename(target, &backup_path)
            .with_context(|| format!("Failed to backup existing item at {:?}", target))?;
            // Propagate error if backup fails.
    }

    // 4. Create the actual symbolic link using platform-specific APIs.
    #[cfg(unix)] // Unix-like systems (Linux, macOS)
    {
        std::os::unix::fs::symlink(source, target).with_context(|| {
            format!("Failed to create symlink from {:?} to {:?}", source, target)
        })?;
        info!("Created symlink: {:?} -> {:?}", target, source);
    }
    #[cfg(windows)] // Windows systems
    {
        // Windows requires different functions for file vs. directory links.
        if source.is_dir() {
            // Create a directory symlink (requires specific privileges on Windows).
            std::os::windows::fs::symlink_dir(source, target).with_context(|| {
                format!(
                    "Failed to create directory symlink from {:?} to {:?}",
                    source, target
                )
            })?;
            info!("Created directory symlink: {:?} -> {:?}", target, source);
        } else {
             // Create a file symlink.
            std::os::windows::fs::symlink_file(source, target).with_context(|| {
                format!(
                    "Failed to create file symlink from {:?} to {:?}",
                    source, target
                )
            })?;
            info!("Created file symlink: {:?} -> {:?}", target, source);
        }
    }
    // Fallback for other platforms where symlink creation isn't automatically handled here.
    #[cfg(not(any(unix, windows)))]
    {
        // Return an error indicating lack of support for this platform.
        bail!("Symlink creation not automatically implemented for this platform.");
    }

    Ok(()) // Symlink created successfully.
}

// --- Unit Tests ---
// Tests remain the same as they cover the function's logic adequately.
#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf}; // Ensure PathBuf is imported here as well.
    use tempfile::tempdir;

    // Basic test: Create a link to a file where the target doesn't exist.
    #[test]
    fn test_create_symlink_basic() -> Result<()> {
        let dir = tempdir()?;
        let source_file = dir.path().join("source.txt");
        let target_link = dir.path().join("target.link");
        fs::write(&source_file, "test")?; // Create the source file.
        create_symlink(&source_file, &target_link)?; // Create the link.
        // Verify link existence and type.
        assert!(target_link.exists(), "Target link should exist");
        assert!(target_link.is_symlink(), "Target should be a symlink");
        // Verify the link points to the correct source file.
        assert_eq!(fs::read_link(&target_link)?, source_file);
        Ok(())
    }

    // Test backing up an existing file at the target location.
    #[test]
    fn test_create_symlink_target_exists_backup() -> Result<()> {
        let dir = tempdir()?;
        let source_file = dir.path().join("source.txt");
        let target_link = dir.path().join("target.link");
        // Construct expected backup path.
        let backup_path = PathBuf::from(format!("{}.devrs_backup", target_link.display()));
        fs::write(&source_file, "source")?; // Create source file.
        fs::write(&target_link, "original target")?; // Create conflicting file at target.
        create_symlink(&source_file, &target_link)?; // Create link (should trigger backup).
        // Verify link creation.
        assert!(target_link.exists());
        assert!(target_link.is_symlink());
        // Verify backup file existence and content.
        assert!(backup_path.exists(), "Backup path should exist");
        assert_eq!(fs::read_to_string(&backup_path)?, "original target");
        // Verify the new link points correctly.
        assert_eq!(fs::read_link(&target_link)?, source_file);
        Ok(())
    }

    // Test idempotency: If the correct link already exists, no backup should occur.
    #[test]
    fn test_create_symlink_already_correct() -> Result<()> {
        let dir = tempdir()?;
        let source_file = dir.path().join("source.txt");
        let target_link = dir.path().join("target.link");
        fs::write(&source_file, "test")?; // Create source.
        // Manually create the *correct* symlink beforehand.
        #[cfg(unix)]
        std::os::unix::fs::symlink(&source_file, &target_link)?;
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&source_file, &target_link)?;
        // Run the function again.
        create_symlink(&source_file, &target_link)?;
        // Assert: No backup file should have been created.
        let backup_path = PathBuf::from(format!("{}.devrs_backup", target_link.display()));
        assert!(!backup_path.exists(), "Backup should not exist");
        Ok(())
    }

    // Test error handling when the source file does not exist.
    #[test]
    fn test_create_symlink_source_missing() {
        let dir = tempdir().unwrap();
        let source_file = dir.path().join("nonexistent_source.txt"); // Source doesn't exist.
        let target_link = dir.path().join("target.link");
        let result = create_symlink(&source_file, &target_link);
        // Assert: Expect an error.
        assert!(result.is_err());
        // Assert: Check the error message content.
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("source path does not exist"));
    }

    // TODO: Add tests for:
    // - Backing up an existing directory at the target.
    // - Backing up an existing *incorrect* symlink at the target.
    // - Creating links within subdirectories (testing parent directory creation).
    // - Permission errors (harder to test reliably across platforms).
    // - Windows directory link creation (if testing on Windows).
}
