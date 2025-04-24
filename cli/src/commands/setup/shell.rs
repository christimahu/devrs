//! # DevRS Shell Setup Handler
//!
//! File: cli/src/commands/setup/shell.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten 
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs setup shell` subcommand, which configures
//! the user's host shell (.bashrc or .zshrc) to integrate with DevRS. It adds
//! shell functions, aliases, and appropriate paths to make DevRS commands
//! available and convenient.
//!
//! ## Architecture
//!
//! The implementation follows these steps:
//!
//! 1. Detect user's shell type (bash, zsh)
//! 2. Determine the appropriate rc file path (~/.bashrc, ~/.zshrc)
//! 3. Find the absolute path to the devrs binary and shell_functions file
//! 4. Define the integration lines to add to the rc file
//! 5. Check if integration is already present
//! 6. Create backup of existing rc file
//! 7. Append integration lines to rc file
//!
//! ## Usage
//!
//! ```bash
//! # Basic shell integration
//! devrs setup shell
//!
//! # Force reinstallation even if already configured
//! devrs setup shell --force
//! ```
//!
//! After running this command, users need to reload their shell configuration
//! (`source ~/.bashrc` or `source ~/.zshrc`) or start a new terminal session.
//!
use crate::core::error::Result; // Use anyhow Result
use anyhow::Context;
use clap::Parser;
use std::env;
use std::path::PathBuf;

/// Arguments for the 'setup shell' subcommand.
#[derive(Parser, Debug)]
pub struct ShellArgs {
    /// Force setup even if configuration seems up-to-date (re-add lines).
    #[arg(long)]
    force: bool,
    // TODO: Add specific shell type override? --shell=bash ?
}

/// Handler function for the 'setup shell' subcommand.
/// Configures the user's host shell (.bashrc or .zshrc) to integrate with DevRS.
/// This involves adding an alias for the `devrs` command (if needed) and sourcing
/// the shared `config/shell_functions` file.
pub async fn handle_shell(args: ShellArgs) -> Result<()> {
    tracing::info!("Handling setup shell command (Force: {})...", args.force);

    // Placeholder logic - Detects shell and path but performs no modifications.
    let shell_type = detect_shell();
    let rc_path = get_rc_path(&shell_type)?;
    tracing::debug!("Detected shell: {}, rc path: {:?}", shell_type, rc_path);

    // Explicitly state that no action is taken yet.
    println!("Warning: 'setup shell' currently only detects the shell type ({}) and rc file path ({:?}).", shell_type, rc_path);
    println!("Warning: It does not yet modify the configuration file.");
    // TODO: Implement the logic to check for existing config, backup, and write new config lines.
    // Consider returning an error or specific status if it shouldn't succeed in this state.
    // anyhow::bail!("'setup shell' is not yet implemented.");
    Ok(())
}

/// Detects the user's default shell type based on the $SHELL environment variable.
fn detect_shell() -> String {
    env::var("SHELL").map_or_else(
        |_| {
            tracing::warn!("Could not detect SHELL env var, defaulting to bash.");
            "bash".to_string() // Default if SHELL is not set
        },
        |shell_path| {
            if shell_path.ends_with("zsh") {
                "zsh".to_string()
            } else if shell_path.ends_with("bash") {
                "bash".to_string()
            } else {
                tracing::warn!(
                    "Unsupported shell detected: {}, defaulting to bash.",
                    shell_path
                );
                "bash".to_string() // Default for others (fish, etc.) for now
            }
        },
    )
}

/// Gets the expected path to the shell configuration file based on shell type.
fn get_rc_path(shell_type: &str) -> Result<PathBuf> {
    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    let rc_filename = match shell_type {
        "zsh" => ".zshrc",
        "bash" => ".bashrc",
        _ => anyhow::bail!("Unsupported shell type for setup: {}", shell_type),
    };
    Ok(home_dir.join(rc_filename))
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    /// Test argument parsing.
    #[test]
    fn test_shell_args_parsing() {
        let args = ShellArgs::try_parse_from(&["shell"]).unwrap(); // Using "shell" as command name for test parsing
        assert!(!args.force);

        let args_force = ShellArgs::try_parse_from(&["shell", "--force"]).unwrap();
        assert!(args_force.force);
    }

    /// TDD: Placeholder test for the handler logic.
    /// Should verify shell detection, rc file reading/writing, backup creation,
    /// and that the correct lines are added.
    #[tokio::test]
    async fn test_handle_shell_logic() {
        let args = ShellArgs { force: false };

        // --- Test Setup ---
        // Requires extensive mocking of env vars ($SHELL, $HOME), fsutils, std::env::current_exe, config loading.
        let temp_home = tempdir().unwrap();
        let dummy_rc_path = temp_home.path().join(".bashrc"); // Assume bash for test
        fs::write(&dummy_rc_path, "# Initial content\n").unwrap();

        // Mock env vars (needs careful handling, e.g., using 'serial_test' crate)
        // MOCK_ENV.set("SHELL", "/bin/bash");
        // MOCK_ENV.set("HOME", temp_home.path().to_str().unwrap());
        // MOCK_CONFIG.set("repo_config_path", "/mock/repo/config"); // Path to find shell_functions

        // Mock fsutils and std::env::current_exe()
        // MOCK_FSUTILS.expect_read_file().returning(|_| Ok("# Initial content\n".to_string()));
        // MOCK_FSUTILS.expect_rename_backup().returning(|_,_| Ok(()));
        // MOCK_FSUTILS.expect_write_file().returning(|_, content| {
        //      assert!(content.contains("alias devrs="));
        //      assert!(content.contains("source \"/mock/repo/config/shell_functions\""));
        //      Ok(())
        // });
        // MOCK_ENV.expect_current_exe().returning(|| Ok(PathBuf::from("/mock/path/to/devrs")));

        // --- Execute Placeholder ---
        let result = handle_shell(args).await;

        // --- Assert ---
        // Real test verifies mock calls and potentially the content written.
        assert!(result.is_ok(), "Placeholder should return Ok"); // Keep simple check for now
    }

    // Note: Testing detect_shell and get_rc_path reliably requires mocking environment variables.
}
