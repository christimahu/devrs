//! # DevRS Config Setup Handler
//!
//! File: cli/src/commands/setup/config.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs setup config` subcommand. Its primary role is
//! to set up the user's core configuration file (`~/.config/devrs/config.toml`) by
//! ensuring the source configuration file (`presets/config.toml`) exists in the repository
//! (copying it from `presets/config_sample.toml` if the active config is missing)
//! and then creating a symbolic link from the standard user configuration path to the
//! repository's source file (`presets/config.toml`).
//!
//! This setup ensures that DevRS commands can find the base configuration, while allowing
//! users to manage their configuration source within the repository and preventing accidental
//! check-ins of personalized settings if `presets/config.toml` is gitignored.
//!
//! ## Architecture / Workflow
//!
//! 1.  **Parse Arguments:** Accepts `ConfigArgs` (currently empty).
//! 2.  **Locate Paths:** Uses `find_repo_root` to find the repository base, then constructs paths to `presets/config.toml` (active config) and `presets/config_sample.toml` (template).
//! 3.  **Check/Create Active Config:**
//!     * If `presets/config.toml` does **not** exist:
//!         * Checks if `presets/config_sample.toml` exists.
//!         * If the sample exists, it's **copied** to `presets/config.toml`.
//!         * If the sample does **not** exist, the setup **fails** with an error, as the sample is required to create the initial active config.
//!     * If `presets/config.toml` **already exists**, it is left untouched, and a message is printed.
//! 4.  **Locate User Config Dir:** Uses the `directories` crate to find the standard user config directory (e.g., `~/.config/devrs/`).
//! 5.  **Ensure User Config Dir:** Creates the user config directory if it doesn't exist using `fsio::ensure_dir_exists`.
//! 6.  **Create Symlink:** Uses `fslinks::create_symlink` to create a symbolic link from `<user_config_dir>/config.toml` pointing to `<repo_root>/presets/config.toml`. This helper handles backing up any existing file/directory at the user config path.
//!
//! ## Usage
//!
//! While this command can be run directly, it's typically invoked as part of `devrs setup all`.
//!
//! ```bash
//! # Run directly (if needed, e.g., after manually deleting the symlink)
//! # Must be run from the root of the devrs repository clone
//! cd /path/to/devrs
//! devrs setup config
//! ```
//!
use crate::{
    commands::setup::find_repo_root, // Use shared helper to find repo root
    common::fs::{io as fsio, links as fslinks}, // Filesystem utilities
    core::error::Result,             // Standard Result type
};
use anyhow::{bail, Context}; // Error handling utilities
use clap::Parser; // Argument parsing
use directories::ProjectDirs; // For finding standard config dirs
use std::fs as std_fs; // Standard filesystem operations (copy)
use tracing::{debug, error, info}; // Logging

// Define the expected names for the config files within the presets directory.
const ACTIVE_CONFIG_FILENAME: &str = "config.toml";
const SAMPLE_CONFIG_FILENAME: &str = "config_sample.toml"; // Name of the template file

/// # Config Setup Arguments (`ConfigArgs`)
///
/// Defines arguments for the `devrs setup config` subcommand.
/// Currently, no specific arguments are needed for this step.
#[derive(Parser, Debug, Default)] // Default derive needed for calling from `all`
#[command(
    name = "config",
    about = "Set up the user configuration symlink for DevRS",
    long_about = "Ensures the source config exists (copying from sample if needed) and creates a symlink\n\
                  from ~/.config/devrs/config.toml to <repo>/presets/config.toml."
)]
pub struct ConfigArgs {
    // No arguments currently needed for this specific setup step.
    // Could add --force later if needed to force re-symlinking without backup handling differences.
}

/// # Handle Config Setup Command (`handle_config`)
///
/// Asynchronous handler function that orchestrates the `config.toml` setup.
/// Implements the copy-from-sample logic and creates the user config symlink.
///
/// ## Arguments
///
/// * `_args`: The parsed `ConfigArgs` (currently unused).
///
/// ## Returns
///
/// * `Result<()>`: `Ok(())` on successful setup of the config file and symlink.
/// * `Err`: If essential paths cannot be determined, the required `config_sample.toml` is missing,
///   or if filesystem operations (copy, directory creation, symlink) fail.
pub async fn handle_config(_args: ConfigArgs) -> Result<()> {
    info!("Handling setup config command...");

    // Find the repository root directory using the shared helper.
    let repo_root = find_repo_root()?;
    let presets_dir = repo_root.join("presets"); // Define presets directory
    let repo_config_path = presets_dir.join(ACTIVE_CONFIG_FILENAME); // Path for the active config
    let repo_config_sample_path = presets_dir.join(SAMPLE_CONFIG_FILENAME); // Path for the template

    // Ensure presets directory exists (important if cloning fresh).
    fsio::ensure_dir_exists(&presets_dir).context("Failed to ensure presets directory exists")?;

    debug!("Presets dir: {}", presets_dir.display());
    debug!("Active config path: {}", repo_config_path.display());
    debug!("Sample config path: {}", repo_config_sample_path.display());

    // --- Check/Create active config.toml in presets directory ---
    if !repo_config_path.exists() {
        // Active config.toml does NOT exist. Try creating from sample.
        info!("No presets/config.toml found.");
        // Check if the sample file exists.
        if repo_config_sample_path.is_file() {
            info!(
                "Found {}. Copying it to {}...",
                SAMPLE_CONFIG_FILENAME, ACTIVE_CONFIG_FILENAME
            );
            println!(
                "Creating {} from sample template {}...", // User message
                ACTIVE_CONFIG_FILENAME, SAMPLE_CONFIG_FILENAME
            );
            // Perform the copy operation using standard library fs::copy.
            std_fs::copy(&repo_config_sample_path, &repo_config_path).with_context(|| {
                format!(
                    "Failed to copy sample config from {} to {}",
                    repo_config_sample_path.display(),
                    repo_config_path.display()
                )
            })?;
            info!(
                "Successfully copied sample config to {}",
                repo_config_path.display()
            );
        } else {
            // Sample file is missing, which is an error because we need it to create the active one.
            error!(
                "Required template file {} not found at {}",
                SAMPLE_CONFIG_FILENAME,
                repo_config_sample_path.display()
            );
            // Use anyhow::bail! to return a specific error immediately.
            bail!(
                "Required configuration template '{}' not found in the repository presets.\n\
                 Cannot proceed with setup without this template file.",
                SAMPLE_CONFIG_FILENAME
            );
        }
    } else {
        // Active config.toml already exists. Leave it as is.
        info!(
            "Found existing {}. Leaving it untouched.",
            repo_config_path.display()
        );
        println!(
            // User message
            "Found existing {} in presets/, skipping creation from sample.",
            ACTIVE_CONFIG_FILENAME
        );
    }

    // --- Set up User Config Symlink ---
    // Determine the standard platform-specific user configuration directory path.
    let Some(proj_dirs) = ProjectDirs::from("com", "DevRS", "devrs") else {
        // Error if we can't determine the user config directory (e.g., unsupported OS, weird env).
        bail!("Could not determine standard user config directory.");
    };
    let user_config_dir = proj_dirs.config_dir().to_path_buf(); // e.g., ~/.config/devrs/
    let user_config_path = user_config_dir.join(ACTIVE_CONFIG_FILENAME); // e.g., ~/.config/devrs/config.toml

    debug!(
        "Standard user config path target: {}",
        user_config_path.display()
    );

    // Ensure the user configuration directory exists, creating it if necessary.
    fsio::ensure_dir_exists(&user_config_dir).with_context(|| {
        format!(
            "Failed to ensure user config directory exists at {}",
            user_config_dir.display()
        )
    })?;

    // Create the symbolic link from the user path to the repository's active config file.
    // The `create_symlink` utility handles backups of any pre-existing item at `user_config_path`.
    println!(
        "Symlinking user config ({}) -> repo config ({})...",
        user_config_path.display(),
        repo_config_path.display(),
    );
    fslinks::create_symlink(&repo_config_path, &user_config_path).with_context(|| {
        format!(
            "Failed to symlink repo config.toml to user config path {}",
            user_config_path.display()
        )
    })?;
    println!("✅ User config symlink setup complete.");

    Ok(()) // Indicate successful completion.
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    // Unit tests would go here, likely requiring mocking for filesystem operations
    // (fsio::ensure_dir_exists, fslinks::create_symlink, std_fs::copy),
    // find_repo_root, and ProjectDirs to isolate the logic of handle_config.
    // Test scenarios should cover:
    // - config.toml missing, sample exists -> copy happens, symlink happens.
    // - config.toml missing, sample missing -> bail! error triggered.
    // - config.toml exists -> no copy happens, symlink happens.
    // - symlink already exists correctly -> create_symlink handles idempotency.
    // - symlink exists incorrectly or target is a file -> create_symlink handles backup.
}
