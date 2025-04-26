//! # DevRS Setup Command Group
//!
//! File: cli/src/commands/setup/mod.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module defines the command-line structure and acts as the main dispatcher
//! for the `devrs setup` command group. The primary goal of `devrs setup` is to
//! configure the user's host system to work effectively with DevRS. This involves
//! several key tasks performed by its subcommands:
//!
//! - **Checking Dependencies (`deps`):** Verifying essential tools like Docker, Git, and Neovim are installed.
//! - **Setting up Configuration (`config`):** Ensuring the user-specific `config.toml` symlink points to the repository's configuration (`presets/config.toml`).
//! - **Shell Integration (`integrate`):** Providing instructions to make DevRS shell functions (`presets/shell_functions`) accessible.
//! - **Neovim Configuration (`nvim`):** Setting up Neovim to use the DevRS configuration (`presets/init.lua`), including managing the Packer plugin manager.
//!
//! The `all` subcommand orchestrates these steps in sequence.
//!
//! **Important:** All setup commands must be run from the root of the DevRS repository clone.
//!
//! ## Architecture
//!
//! - **`SetupArgs`**: Parses the top-level `devrs setup` command and determines which subcommand (if any) was invoked using `clap`.
//! - **`SetupCommand`**: An enum representing the available subcommands (`all`, `config`, `deps`, `integrate`, `nvim`).
//! - **`handle_setup`**: The main asynchronous function that first verifies the command is run from the repository root (using `find_repo_root`), then receives the parsed arguments and routes execution to the appropriate subcommand handler. If no subcommand is specified, it defaults to executing the `all` subcommand.
//! - **`find_repo_root`**: A crate-public helper function (defined within this module) used by submodules to locate the repository root by searching upwards for `Cargo.toml` containing `[workspace]`.
//! - **Submodules**: Each subcommand (`all`, `config`, `deps`, `integrate`, `nvim`) has its logic implemented in a corresponding submodule within this directory.
//!
//! ## Usage
//!
//! ```bash
//! # Change to the devrs repository directory first!
//! cd /path/to/devrs
//!
//! # Run the default 'all' setup
//! devrs setup
//!
//! # Run specific setup tasks
//! devrs setup deps
//! devrs setup config
//! devrs setup integrate
//! devrs setup nvim
//! ```
//!
use crate::core::error::Result; // Standard Result type for error handling.
use anyhow::{bail, Context}; // Error context handling and early returns.
use clap::{Parser, Subcommand}; // For argument parsing.
use std::{env, fs, path::PathBuf}; // Standard library modules for env vars, filesystem, paths.
use tracing::{debug, warn}; // Logging facade.

// Declare subcommand modules within the 'setup' group.
mod all;
mod config; // Added config module
mod deps;
mod integrate;
mod nvim;

/// # Setup Command Arguments (`SetupArgs`)
///
/// Parses arguments for the `devrs setup` command group using `clap`.
/// It primarily identifies which specific setup subcommand the user wants to run.
/// If no subcommand is provided, the `handle_setup` function defaults to running the `all` command.
#[derive(Parser, Debug)]
#[command(
    name = "setup", // Command name used in help messages.
    about = "Configure the host system for DevRS (dependencies, config, shell integration, nvim)", // Short help description.
    long_about = "Performs initial setup tasks required for DevRS to function correctly.\n\
                  Checks dependencies, sets up the user configuration file (config.toml symlink),\n\
                  provides shell integration instructions, and configures Neovim.\n\n\
                  IMPORTANT: Must be run from the root of the DevRS repository clone.\n\
                  If no subcommand is specified, 'all' is executed by default." // Longer help description.
)]
pub struct SetupArgs {
    /// The specific setup task to perform (e.g., `deps`, `config`, `nvim`). Defaults to `all` if omitted.
    #[command(subcommand)] // Indicates this field represents a subcommand.
    command: Option<SetupCommand>,
}

/// # Setup Subcommands (`SetupCommand`)
///
/// Enumerates the available subcommands under `devrs setup` using `clap`.
/// Each variant holds the specific arguments structure required by its corresponding handler.
#[derive(Subcommand, Debug)]
enum SetupCommand {
    /// Perform all setup tasks in sequence (default action).
    #[command(about = "Perform all setup tasks (deps, config, integrate, nvim)")]
    All(all::AllArgs), // Contains arguments specific to the 'all' command (e.g., --force-nvim).

    /// Set up the user configuration file (config.toml symlink).
    #[command(about = "Set up the user configuration file (config.toml symlink)")]
    Config(config::ConfigArgs), // Contains arguments for 'config' (currently none).

    /// Check required host system dependencies.
    #[command(about = "Check required host system dependencies (e.g., Docker, Git, Nvim)")]
    Deps(deps::DepsArgs), // Contains arguments for 'deps' (currently none).

    /// Provide instructions for shell integration.
    #[command(about = "Provide instructions for shell integration (sourcing DevRS functions)")]
    Integrate(integrate::IntegrateArgs), // Contains arguments for 'integrate' (currently none).

    /// Set up Neovim configuration and plugins.
    #[command(about = "Set up Neovim configuration (symlink init.lua, install Packer)")]
    Nvim(nvim::NvimArgs), // Contains arguments for 'nvim' (e.g., --force, --skip-plugins).
}

/// # Handle Setup Command (`handle_setup`)
///
/// Asynchronous entry point and dispatcher for the `devrs setup` command group.
/// It first verifies it's being run from the repository root using `find_repo_root`.
/// Then, it determines which subcommand was invoked (defaulting to `all`), and dispatches
/// execution to the appropriate asynchronous handler function from the submodules.
///
/// ## Arguments
///
/// * `args`: The parsed `SetupArgs` struct containing the optional subcommand information.
///
/// ## Returns
///
/// * `Result<()>`: Propagates the `Result` from the executed subcommand handler.
///   `Ok(())` on success, `Err` on failure (including if not run from repo root or if a handler fails).
pub async fn handle_setup(args: SetupArgs) -> Result<()> {
    // --- Verify Running from Repository Root ---
    // This check is crucial because setup commands need access to files within the repo (e.g., presets).
    let repo_root = find_repo_root()?; // Attempt to find the root.
    let essential_check_path = repo_root.join("Cargo.toml"); // Use root Cargo.toml as marker.

    // Check if the marker file exists at the determined root path.
    if !essential_check_path.is_file() {
        // If the marker isn't found, return an error telling the user to run from the repo root.
        bail!(
            "Repository marker file ({}) not found searching upwards from current directory ({}).\n\
             Please run 'devrs setup' commands from within the DevRS repository clone.",
            essential_check_path.file_name().unwrap_or_default().to_string_lossy(), // Show just filename.
            env::current_dir()?.display() // Show starting directory for context.
        );
    }
    // Log success if the check passes.
    debug!(
        "Repository root check passed (found {} at {}).",
        essential_check_path.display(), // Corrected log message to show file path
        repo_root.display() // Show the determined root directory
    );
    // --- End Repository Root Check ---

    // Determine the command to execute. If the user didn't specify one via the command line,
    // default to the 'All' variant using its default arguments.
    let command = args
        .command
        .unwrap_or_else(|| SetupCommand::All(all::AllArgs::default()));

    // Match the determined command and call its corresponding handler function.
    // The `await?` pattern awaits the async handler and propagates any errors upward.
    match command {
        SetupCommand::All(args) => all::handle_all(args).await?,
        SetupCommand::Config(args) => config::handle_config(args).await?, // Added dispatch for config
        SetupCommand::Integrate(args) => integrate::handle_integrate(args).await?,
        SetupCommand::Nvim(args) => nvim::handle_nvim(args).await?,
        SetupCommand::Deps(args) => deps::handle_deps(args).await?,
    }

    // If the handler completed successfully (didn't return Err), return Ok.
    Ok(())
}

/// # Find Repository Root (`find_repo_root`)
///
/// Attempts to determine the root directory of the DevRS repository by searching
/// upwards from the current working directory for the main `Cargo.toml` file
/// containing the `[workspace]` definition. Made `pub(crate)` to be accessible
/// by submodules within the `setup` command group.
///
/// ## Returns
///
/// * `Result<PathBuf>`: The absolute path to the determined repository root on success.
/// * `Err`: If the current working directory cannot be obtained, filesystem errors
///   occur during search, or the workspace root marker is not found before reaching
///   the filesystem root.
pub(crate) fn find_repo_root() -> Result<PathBuf> {
    // Get the directory where the command was run.
    let start_dir = env::current_dir()
        .context("Failed to get current directory to start repo root search")?;
    let mut current_dir = start_dir.clone(); // Clone to modify in the loop.

    // Loop upwards through parent directories.
    loop {
        // Construct the path to a potential Cargo.toml in the current directory.
        let cargo_toml_path = current_dir.join("Cargo.toml");
        // Check if it exists and is a file.
        if cargo_toml_path.is_file() {
            // If it's a file, try to read it.
            match fs::read_to_string(&cargo_toml_path) {
                Ok(content) => {
                    // Check if the content contains the workspace marker.
                    if content.contains("[workspace]") {
                        debug!("Found workspace root marker at: {}", current_dir.display());
                        // Found the root, return the current directory path.
                        return Ok(current_dir);
                    }
                    // If it's a Cargo.toml but not the workspace root, continue searching upwards.
                }
                Err(e) => {
                    // Failed to read the file (e.g., permissions). Log warning and continue search.
                    warn!(
                        "Could not read potential root Cargo.toml at {}: {}",
                        cargo_toml_path.display(),
                        e
                    );
                }
            }
        }

        // Move up to the parent directory for the next iteration.
        if let Some(parent) = current_dir.parent() {
            // Safety check: stop if we are trying to move up from the root itself.
            if parent == current_dir {
                break;
            }
            current_dir = parent.to_path_buf(); // Update current directory for next loop.
        } else {
            // Reached the filesystem root without finding the marker.
            break;
        }
    }
    // If the loop finishes without finding the root, return an error.
    bail!(
        "Could not find repository root (marker '[workspace]' in Cargo.toml) searching upwards from starting directory: {}",
        start_dir.display()
    );
}

// --- Unit Tests --- (Add test for `config` subcommand parsing)
#[cfg(test)]
mod tests {
    use super::*; // Import items from the parent module (setup/mod.rs)
    use tempfile::tempdir;

    #[test]
    fn test_parses_setup_default_to_all() {
        let args = SetupArgs::try_parse_from(&["setup"]).unwrap();
        assert!(args.command.is_none());
    }

    #[test]
    fn test_parses_setup_all() {
        let args = SetupArgs::try_parse_from(&["setup", "all"]).unwrap();
        assert!(matches!(args.command, Some(SetupCommand::All(_))));
    }

    #[test]
    fn test_parses_setup_config() { // Test for new subcommand
        let args = SetupArgs::try_parse_from(&["setup", "config"]).unwrap();
        assert!(matches!(args.command, Some(SetupCommand::Config(_))));
    }

    #[test]
    fn test_parses_setup_deps() {
        let args = SetupArgs::try_parse_from(&["setup", "deps"]).unwrap();
        assert!(matches!(args.command, Some(SetupCommand::Deps(_))));
    }

    #[test]
    fn test_parses_setup_integrate() {
        let args = SetupArgs::try_parse_from(&["setup", "integrate"]).unwrap();
        assert!(matches!(args.command, Some(SetupCommand::Integrate(_))));
    }

    #[test]
    fn test_parses_setup_nvim() {
        let args =
            SetupArgs::try_parse_from(&["setup", "nvim", "--force", "--skip-plugins"]).unwrap();
        assert!(matches!(args.command, Some(SetupCommand::Nvim(_))));
        if let Some(SetupCommand::Nvim(nvim_args)) = args.command {
            assert!(nvim_args.force);
            assert!(nvim_args.skip_plugins);
        } else {
            panic!("Parsed as wrong variant");
        }
    }

    #[test]
    #[ignore] // Still requires CWD manipulation / mocking
    fn test_find_repo_root_logic() -> Result<()> {
        // --- Setup ---
        let temp_base = tempdir()?;
        let expected_repo_root_path = temp_base.path().join("devrs_test_repo");
        let cli_dir = expected_repo_root_path.join("cli");
        fs::create_dir_all(&cli_dir)?;
        fs::write(
            expected_repo_root_path.join("Cargo.toml"),
            "[workspace]\nmembers=[\"cli\"]",
        )?;
        fs::write(cli_dir.join("main.rs"), "fn main() {}")?;
        let original_cwd = env::current_dir()?;

        // --- Test: Run from repo root ---
        env::set_current_dir(&expected_repo_root_path)?;
        let found_root_1 = find_repo_root()?;
        let canonical_expected_1 = fs::canonicalize(&expected_repo_root_path)?;
        let canonical_found_1 = fs::canonicalize(&found_root_1)?;
        assert_eq!(canonical_found_1, canonical_expected_1);
        env::set_current_dir(&original_cwd)?;

        // --- Test: Run from subdirectory ---
        env::set_current_dir(&cli_dir)?;
        let found_root_2 = find_repo_root()?;
        let canonical_expected_2 = fs::canonicalize(&expected_repo_root_path)?;
        let canonical_found_2 = fs::canonicalize(&found_root_2)?;
        assert_eq!(canonical_found_2, canonical_expected_2);
        env::set_current_dir(&original_cwd)?;

        // --- Test: Run from outside (should fail) ---
        let outside_dir = temp_base.path().join("outside");
        fs::create_dir(&outside_dir)?;
        env::set_current_dir(&outside_dir)?;
        let result_fail = find_repo_root();
        assert!(result_fail.is_err());
        if let Err(e) = result_fail {
             assert!(e.to_string().contains("Could not find repository root"));
        }
        env::set_current_dir(&original_cwd)?;

        Ok(())
    }


    #[tokio::test]
    #[ignore] // Requires mocking or CWD manipulation and marker files
    async fn test_handle_setup_repo_check() {
        // Conceptual test remains the same, checking the guard condition in handle_setup.
    }
}
