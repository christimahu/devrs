//! # DevRS Dependencies Check Handler
//!
//! File: cli/src/commands/setup/deps.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs setup deps` subcommand.
//! It checks if required host system dependencies (Docker, Git, Nvim)
//! are installed and available in the system's PATH. It does **not**
//! attempt to install missing dependencies.
//!
//! ## Architecture
//!
//! 1. Defines the list of required external commands (`REQUIRED_DEPS`).
//! 2. Iterates through the list, calling the internal `check_dependency_exists` helper for each.
//! 3. `check_dependency_exists` uses `std::process::Command` to attempt running the command
//!    (e.g., `docker --version`) and checks the execution result to determine availability.
//! 4. Reports the status (found/missing) for each dependency to the console.
//! 5. If any dependencies are missing, returns an error instructing the user
//!    to install them manually.
//!
//! ## Error Handling
//! The command fails if any required dependency is not found or if checking a dependency
//! results in an unexpected execution error (e.g., permission denied).
//!
//! ## Usage
//!
//! ```bash
//! # Must be run from the root of the devrs repository clone
//! cd /path/to/devrs
//! # Check for required dependencies
//! devrs setup deps
//! ```
//!
use crate::core::error::Result; // Use standard Result type
use anyhow::{bail, Context}; // For concise error returns and context
use clap::Parser; // For argument parsing
use std::{
    io::Write as IoWrite,      // Needed for stdout().flush()
    process::{Command, Stdio}, // Used for executing external commands to check existence
};
use tracing::{debug, error, info}; // Logging

// Define required dependencies as a constant array for easy modification.
const REQUIRED_DEPS: [&str; 3] = ["docker", "git", "nvim"];

/// # Dependencies Check Arguments (`DepsArgs`)
///
/// Defines arguments for the `devrs setup deps` subcommand.
/// Currently, no arguments are needed as it only performs checks.
#[derive(Parser, Debug, Default)] // Default derive needed for use in `all` handler
#[command(
    name = "deps", // Explicitly set command name for help text
    about = "Check if required host dependencies (Docker, Git, Nvim) are installed",
    long_about = "Verifies that essential external tools needed by DevRS (Docker, Git, Nvim)\n\
                  are installed and accessible in the system's PATH.\n\
                  This command does NOT attempt to install missing dependencies."
)]
pub struct DepsArgs {
    // No arguments needed for checking only.
}

/// # Handle Dependencies Check Command (`handle_deps`)
///
/// Asynchronous handler function for the 'setup deps' subcommand.
/// Checks if required host system dependencies (Docker, Git, Nvim) are installed and available.
///
/// ## Arguments
///
/// * `_args`: The parsed `DepsArgs` (currently unused).
///
/// ## Returns
///
/// * `Result<()>`: `Ok(())` if all dependencies are found.
/// * `Err`: If any dependency is missing or checking fails.
pub async fn handle_deps(_args: DepsArgs) -> Result<()> {
    info!("Handling setup deps command..."); // Log entry

    // Store names of missing dependencies.
    let mut missing_deps = Vec::new();

    println!("Checking required host dependencies...");
    // Check each dependency defined in the constant array.
    for dep in REQUIRED_DEPS {
        // Use print! without newline initially.
        print!("  - Checking for '{}'... ", dep);
        // Flush stdout to ensure the "Checking..." message appears before the result.
        std::io::stdout()
            .flush()
            .context("Failed to flush stdout")?;

        // Perform the check and handle the result.
        match check_dependency_exists(dep).await {
            Ok(true) => {
                println!("Found."); // Dependency found.
            }
            Ok(false) => {
                println!("Missing."); // Dependency not found.
                missing_deps.push(dep); // Add to missing list.
            }
            Err(e) => {
                // Error occurred during the check. Treat as missing.
                println!("Error checking ({})", e); // Print simple error for user.
                error!("Error checking for dependency '{}': {}", dep, e); // Log detailed error.
                missing_deps.push(dep); // Assume missing if check fails.
            }
        }
    }

    // Report final status based on whether any dependencies were missing.
    if missing_deps.is_empty() {
        println!("✅ All required dependencies found.");
        Ok(()) // Success.
    } else {
        // Dependencies are missing, return an error with instructions.
        let missing_list = missing_deps.join(", ");
        error!("Missing dependencies: {}", missing_list);
        // Use anyhow::bail! for a clean error return to main.
        bail!(
            "Missing required dependencies: {}. Please install them manually.",
            missing_list
        );
    }
}

/// # Check Dependency Existence (`check_dependency_exists`)
///
/// Checks if a given command name corresponds to an executable found in the system's PATH.
/// Uses a basic, platform-aware approach by trying to run the command with a simple
/// flag (like `--version`) and checking the execution result.
/// This is a basic implementation used locally within the setup module.
///
/// ## Arguments
///
/// * `cmd_name`: The name of the command to check (e.g., "docker").
///
/// ## Returns
///
/// * `Result<bool>`: `Ok(true)` if the command appears to exist and run without OS error,
///   `Ok(false)` if the OS reports it as "Not Found".
/// * `Err`: If executing the check command itself fails for other reasons (e.g., permissions).
async fn check_dependency_exists(cmd_name: &str) -> Result<bool> {
    info!("Performing basic check for command: {}", cmd_name);
    // Choose a simple, common flag that usually exits quickly.
    let version_flag = "--version";

    let mut command = Command::new(cmd_name);
    command.arg(version_flag);
    command.stdout(Stdio::null()); // Discard standard output.
    command.stderr(Stdio::null()); // Discard standard error.

    debug!("Running check: {} {}", cmd_name, version_flag);

    // Execute the command and check its status.
    // We run this synchronously as it's expected to be very fast.
    match command.status() {
        Ok(status) => {
            // Command executed. It exists in PATH.
            debug!(
                "Check command '{} {}' executed with status: {}",
                cmd_name, version_flag, status
            );
            // Consider successful execution (even non-zero exit for --version) as found.
            Ok(true)
        }
        Err(e) => {
            // An error occurred trying to *run* the command.
            if e.kind() == std::io::ErrorKind::NotFound {
                // The OS explicitly couldn't find the command.
                debug!("Command '{}' not found (ErrorKind::NotFound).", cmd_name);
                Ok(false)
            } else {
                // Another error occurred (e.g., permission denied).
                error!(
                    "Error executing check command '{} {}': {}",
                    cmd_name, version_flag, e
                );
                // Propagate the error.
                Err(anyhow::Error::new(e).context(format!(
                    "Failed to execute command check for '{}'",
                    cmd_name
                )))
            }
        }
    }
}

// --- Unit Tests ---
/// Tests for the `setup deps` subcommand argument parsing and dependency checking logic.
#[cfg(test)]
mod tests {
    use super::*; // Import items from parent module (deps.rs)

    /// Test argument parsing (currently no arguments).
    #[test]
    fn test_deps_args_parsing() {
        // Try parsing the command "deps".
        let args = DepsArgs::try_parse_from(["deps"]).unwrap();
        // Assertions can be added here if arguments are introduced later.
        let _ = args; // Use args to prevent unused variable warning
    }

    /// Test the dependency check function (`check_dependency_exists`).
    /// Relies on `git` being present and a fake command not being present.
    #[tokio::test]
    async fn test_check_dependency_exists_logic() {
        // Test for 'git', which is required and should generally be present.
        let git_exists = check_dependency_exists("git").await.unwrap_or(false);
        assert!(
            git_exists,
            "'git' should be found in PATH for test execution"
        );

        // Test for a command highly unlikely to exist.
        let non_existent_cmd = "nonexistent_devrs_test_command_98765";
        let non_existent_exists = check_dependency_exists(non_existent_cmd)
            .await
            .unwrap_or(true);
        assert!(
            !non_existent_exists,
            "'{}' should not be found",
            non_existent_cmd
        );
    }

    /// Test the main handler logic (`handle_deps`).
    /// Requires mocking `check_dependency_exists` for reliable test results.
    #[tokio::test]
    #[ignore] // Needs mocking `check_dependency_exists` for reliable testing.
    async fn test_handle_deps_logic() {
        // --- Mocking Setup (Conceptual - requires a mocking framework) ---
        // mock_check_dependency_exists("docker") -> returns Ok(true)
        // mock_check_dependency_exists("git") -> returns Ok(true)
        // mock_check_dependency_exists("nvim") -> returns Ok(true)

        // --- Test Execution (All Found) ---
        let args_ok = DepsArgs::default();
        let result_ok = handle_deps(args_ok).await;

        // --- Assertions (All Found) ---
        assert!(
            result_ok.is_ok(),
            "handle_deps should succeed if all dependencies are found"
        );
        // TODO: Capture stdout/logs to verify "Found." messages and final success message.

        // --- Mocking Setup (One Missing) ---
        // mock_check_dependency_exists("docker") -> returns Ok(false)
        // mock_check_dependency_exists("git") -> returns Ok(true)
        // mock_check_dependency_exists("nvim") -> returns Ok(true)

        // --- Test Execution (One Missing) ---
        let args_missing = DepsArgs::default();
        let result_missing = handle_deps(args_missing).await;

        // --- Assertions (One Missing) ---
        assert!(
            result_missing.is_err(),
            "handle_deps should fail if a dependency is missing"
        );
        // Check the error message content.
        assert!(
            result_missing
                .unwrap_err()
                .to_string()
                .contains("Missing required dependencies: docker"),
            "Error message should list missing 'docker'"
        );
        // TODO: Capture stdout/logs to verify "Missing." message for docker.
    }
}
