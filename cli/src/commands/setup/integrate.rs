//! # DevRS Shell Integration Instruction Handler
//!
//! File: cli/src/commands/setup/integrate.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs setup integrate` subcommand.
//! Its purpose is **not** to modify shell configuration files directly, but rather
//! to **display the necessary `source` command** that the user should manually add
//! to their shell configuration file (`.bashrc` or `.zshrc`) to enable DevRS
//! shell functions (`presets/shell_functions`). This approach prioritizes safety by
//! avoiding automatic modification of potentially complex user configuration files.
//!
//! ## Architecture
//!
//! 1.  Detects the user's default shell (`bash` or `zsh`) using the `$SHELL` environment variable via the internal `detect_shell` helper.
//! 2.  Determines the absolute path to the `presets/shell_functions` file within
//!     the DevRS repository installation using the `find_repo_root` helper from the `setup` module.
//! 3.  Prints clear instructions to the console, including:
//!     * The exact `source "<path_to_shell_functions>"` command to add.
//!     * Which file (`~/.bashrc` or `~/.zshrc`) the user should add it to based on the detected shell.
//!     * How to reload the shell configuration afterwards (by running `source` again or opening a new terminal).
//!
//! ## Error Handling
//! The command fails if it cannot detect the shell type, find the repository root,
//! locate the `shell_functions` file, or canonicalize the path to it.
//!
//! ## Usage
//!
//! ```bash
//! # Must be run from the root of the devrs repository clone
//! cd /path/to/devrs
//! # Display shell integration instructions
//! devrs setup integrate
//! ```
//!
//! The user then manually copies the displayed `source` command and adds it to their
//! shell configuration file.
//!
use crate::commands::setup::find_repo_root; // Use shared helper from setup::mod
use crate::core::error::Result; // Standard Result type
use anyhow::{bail, Context}; // Error handling utilities
use clap::Parser; // Argument parsing
use std::{
    env, // For SHELL environment variable
    fs, // For canonicalize
};
use tracing::{debug, info, warn}; // Logging

/// # Integrate Shell Arguments (`IntegrateArgs`)
///
/// Defines arguments for the `devrs setup integrate` subcommand.
/// Currently, no arguments are needed as it only displays instructions.
#[derive(Parser, Debug, Default)] // Default derive needed for setup all
#[command(
    name = "integrate", // Explicit command name for help
    about = "Show instructions to integrate DevRS functions into your shell",
    long_about = "Displays the command needed to source DevRS shell functions.\n\
                  You must manually add this command to your shell configuration file\n\
                  (e.g., ~/.bashrc or ~/.zshrc) and reload your shell."
)]
pub struct IntegrateArgs {
    // No arguments currently needed.
}

/// # Handle Shell Integration Command (`handle_integrate`)
///
/// Asynchronous handler for `devrs setup integrate`. Determines the path to `shell_functions`
/// and prints instructions for the user to manually source it. Does not modify any user files.
///
/// ## Arguments
///
/// * `_args`: The parsed `IntegrateArgs` (currently unused).
///
/// ## Returns
///
/// * `Result<()>`: `Ok(())` on success (instructions printed).
/// * `Err`: If the shell cannot be detected, the repo root cannot be found,
///   or the `shell_functions` file is missing or inaccessible.
pub async fn handle_integrate(_args: IntegrateArgs) -> Result<()> {
    info!("Handling setup integrate command...");

    // --- 1. Detect Shell ---
    // Detect the shell primarily to tell the user *which* rc file to edit.
    let shell_type = detect_shell()?;
    let rc_filename = match shell_type.as_str() {
        "bash" => "~/.bashrc",
        "zsh" => "~/.zshrc",
        // Provide a generic message if the shell is unknown/unsupported.
        _ => "~/.[your_shell_rc_file]",
    };
    info!(
        "Detected shell: {}. User should edit: {}",
        shell_type, rc_filename
    );

    // --- 2. Determine Path to shell_functions ---
    // Find the repository root directory using the shared helper from setup::mod.
    let repo_root = find_repo_root()?;
    // Path to the functions file within the 'presets' directory.
    let shell_functions_path = repo_root.join("presets").join("shell_functions");

    // Verify the shell_functions file exists and is a file.
    if !shell_functions_path.is_file() {
        bail!(
            "DevRS shell functions file not found at expected location: {}. Cannot provide setup instructions.",
            shell_functions_path.display()
        );
    }
    debug!("Found shell_functions at: {}", shell_functions_path.display());

    // Use canonicalize to get a reliable absolute path for the source command.
    // This resolves symlinks and cleans up the path representation (e.g., removes "..").
    let canonical_functions_path = fs::canonicalize(&shell_functions_path).with_context(|| {
        format!(
            "Failed to get canonical path for shell_functions file: {}",
            shell_functions_path.display()
        )
    })?;
    debug!(
        "Canonical path for sourcing: {}",
        canonical_functions_path.display()
    );

    // --- 3. Print Instructions ---
    // Print clear, multi-line instructions for the user.
    println!("\n--- DevRS Integration Instructions ---");
    println!("\nTo make DevRS shell functions available, please **manually add** the following");
    println!("line to your shell configuration file ({})", rc_filename);
    println!("and then reload your shell or open a new terminal:");
    // Add visual separators for clarity.
    println!("\n# --- Start: Add this line to {} ---", rc_filename);
    // Print the exact command the user needs, using quotes for safety,
    // especially if paths might contain spaces or special characters.
    println!("source \"{}\"", canonical_functions_path.display());
    println!("# --- End: Add this line ---");

    println!("\nAfter adding the line, reload your configuration by running:");
    println!("  source {}", rc_filename);
    println!("Or simply open a new terminal window.");
    println!("------------------------------------------");

    Ok(()) // Indicate instructions were successfully displayed.
}

/// # Detect Shell (`detect_shell`)
///
/// Detects the user's default shell type ('bash', 'zsh', or 'unknown')
/// based on the `$SHELL` environment variable. This helps provide the
/// correct filename suggestion to the user (e.g., `.bashrc` vs `.zshrc`).
///
/// ## Returns
///
/// * `Result<String>`: The detected shell name ("bash", "zsh", "unknown").
///   It returns `Ok("bash")` if `$SHELL` is not set, assuming bash/zsh compatibility
///   for the `source` command instruction.
fn detect_shell() -> Result<String> {
    info!("Detecting shell type (using basic $SHELL check)...");
    // Read the SHELL environment variable.
    match env::var("SHELL") {
        Ok(shell_path) => {
            // Check the end of the path for known shell names.
            if shell_path.ends_with("zsh") {
                debug!("Detected zsh from SHELL={}", shell_path);
                Ok("zsh".to_string())
            } else if shell_path.ends_with("bash") {
                debug!("Detected bash from SHELL={}", shell_path);
                Ok("bash".to_string())
            } else {
                // If it's neither bash nor zsh, log a warning and return "unknown".
                warn!(
                    "Unsupported shell detected via $SHELL: {}. Providing generic instructions.",
                    shell_path
                );
                Ok("unknown".to_string()) // Indicate unsupported shell.
            }
        }
        Err(_) => {
            // If $SHELL is not set, log a warning and assume a common default.
            // The instructions will still be generally useful for bash/zsh users.
            warn!("$SHELL environment variable not found. Assuming bash/zsh compatible instructions.");
            Ok("bash".to_string()) // Default for message formatting.
        }
    }
}

// --- Unit Tests ---
/// Tests for the `setup integrate` subcommand's argument parsing and logic.
#[cfg(test)]
mod tests {
    use super::*; // Import items from parent module (integrate.rs)
    use std::{env, fs}; // Filesystem operations for testing
    use tempfile::tempdir; // Create temporary directories

    /// Test argument parsing (currently no arguments).
    #[test]
    fn test_integrate_args_parsing() {
        let args = IntegrateArgs::try_parse_from(["integrate"]).unwrap();
        // No arguments to assert currently. Test ensures parsing doesn't fail.
        let _ = args; // Use args to prevent unused variable warning
    }

    /// Test shell detection logic based on environment variables.
    /// Note: Modifies environment variables; use `serial_test` crate if running tests in parallel.
    #[test]
    fn test_detect_shell_logic() {
        // Backup original SHELL if exists.
        let original_shell = std::env::var("SHELL").ok();

        // Test Zsh detection.
        std::env::set_var("SHELL", "/usr/bin/zsh");
        assert_eq!(detect_shell().unwrap(), "zsh");

        // Test Bash detection.
        std::env::set_var("SHELL", "/bin/bash");
        assert_eq!(detect_shell().unwrap(), "bash");

        // Test unsupported shell detection.
        std::env::set_var("SHELL", "/bin/fish");
        assert_eq!(detect_shell().unwrap(), "unknown"); // Expected "unknown"

        // Test when $SHELL is unset.
        std::env::remove_var("SHELL");
        assert_eq!(detect_shell().unwrap(), "bash"); // Defaults to "bash"

        // Restore original SHELL.
        if let Some(shell) = original_shell {
            std::env::set_var("SHELL", shell);
        } else {
            std::env::remove_var("SHELL");
        }
    }

    /// Test the main handler logic (`handle_integrate`).
    /// Creates a temporary directory structure, sets the CWD to mimic
    /// running from the repo root, and verifies that the handler runs without error.
    /// It does not capture stdout to verify the output content, but checks for success.
    #[tokio::test]
    async fn test_handle_integrate_runs_ok() -> Result<()> {
        // --- Setup ---
        let temp_dir = tempdir()?; // Create a temporary directory.
        let repo_root = temp_dir.path();

        // Create the root marker file needed by find_repo_root
        fs::write(repo_root.join("Cargo.toml"), "[workspace]\nmembers=[\"cli\"]")?;
        // Create the necessary dummy preset structure within the temp dir.
        let presets_dir = repo_root.join("presets");
        fs::create_dir(&presets_dir)?;
        let functions_path = presets_dir.join("shell_functions");
        fs::write(&functions_path, "#!/bin/bash\n echo 'DevRS Funcs'")?;

        // Set CWD to the temp repo root for find_repo_root() helper to work.
        // **WARNING:** Affects process CWD. Use `serial_test` if needed.
        let original_cwd = env::current_dir()?;
        env::set_current_dir(repo_root)?;

        // --- Test Execution ---
        let args = IntegrateArgs::default();
        let result = handle_integrate(args).await;

        // --- Assertions ---
        assert!(
            result.is_ok(),
            "handle_integrate should succeed if shell_functions file exists: {:?}",
            result // Print error if it fails
        );
        // To verify output, stdout capture would be needed.

        // --- Cleanup ---
        // Restore original CWD.
        env::set_current_dir(original_cwd)?;
        // Temp dir guard handles directory removal automatically when it goes out of scope.

        Ok(())
    }

    /// Test that `handle_integrate` fails if `shell_functions` is missing.
    #[tokio::test]
    async fn test_handle_integrate_fails_if_functions_missing() -> Result<()> {
        // --- Setup ---
        let temp_dir = tempdir()?;
        let repo_root = temp_dir.path();
        // Create the root marker file needed by find_repo_root
        fs::write(repo_root.join("Cargo.toml"), "[workspace]\nmembers = [\"cli\"]")?;
        // Create the presets dir, but *not* shell_functions inside it
        let presets_dir = repo_root.join("presets");
        fs::create_dir(&presets_dir)?;


        // Set CWD to the temp repo root.
        let original_cwd = env::current_dir()?;
        env::set_current_dir(repo_root)?;

        // --- Test Execution ---
        let args = IntegrateArgs::default();
        let result = handle_integrate(args).await;

        // --- Assertions ---
        assert!(
            result.is_err(),
            "handle_integrate should fail if shell_functions is missing"
        );
        // Optional: Check the specific error message content.
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("shell functions file not found"));

        // --- Cleanup ---
        env::set_current_dir(original_cwd)?;
        Ok(())
    }
}
