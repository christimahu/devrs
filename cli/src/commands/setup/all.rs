//! # DevRS Setup All Handler
//!
//! File: cli/src/commands/setup/all.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs setup all` subcommand, which serves as
//! the **default action** for `devrs setup`. It orchestrates the complete host
//! system setup process by calling the individual setup handlers for dependencies (`deps`),
//! configuration files (`config`), shell integration (`integrate`), and Neovim (`nvim`)
//! in the correct sequence.
//!
//! ## Architecture / Workflow
//!
//! The `handle_all` function executes the following steps sequentially:
//! 1.  Calls `deps::handle_deps` to check for external tools (Docker, Git, Nvim).
//! 2.  Calls `config::handle_config` to set up the user configuration symlink.
//! 3.  Calls `integrate::handle_integrate` to display instructions for sourcing the DevRS shell functions.
//! 4.  Calls `nvim::handle_nvim` to configure Neovim, passing along relevant flags (`--force-nvim`, `--skip-nvim-plugins`).
//! 5.  Prints a final summary message, reminding the user of any manual steps required (like adding the `source` line to their shell config).
//!
//! ## Error Handling
//! If any step encounters an error, the execution stops, and the error is propagated
//! upwards, usually resulting in an error message printed to the console by `main.rs`.
//!
//! ## Usage
//!
//! ```bash
//! # Must be run from the root of the devrs repository clone
//! cd /path/to/devrs
//!
//! # Run the complete setup process (default action)
//! devrs setup
//! # Equivalent explicit command
//! devrs setup all
//!
//! # Run all steps, but force Neovim setup and skip plugin sync
//! devrs setup all --force-nvim --skip-nvim-plugins
//! ```
//!
use crate::{
    commands::setup::{
        config, // Import the new config handler module
        deps,   // Dependency check handler
        integrate, // Shell integration instruction handler
        nvim,   // Neovim setup handler
                // find_repo_root is no longer needed directly here
    },
    core::error::Result, // Standard Result type
};
use anyhow::Context; // Error handling utilities
use clap::Parser; // Argument parsing
// Filesystem, IO, Dirs etc are now primarily handled within the specific handlers
use tracing::info; // Logging

/// # Setup All Arguments (`AllArgs`)
///
/// Defines arguments specific to the `devrs setup all` command using `clap`.
/// Currently, it allows passing flags down specifically to the `nvim` setup step.
#[derive(Parser, Debug, Default)] // Default derive needed for setup/mod.rs default handling
#[command(
    about = "Perform all DevRS setup tasks (deps, config, integrate, nvim)",
    long_about = "Runs all necessary setup steps in sequence:\n\
                  1. Checks host dependencies (Docker, Git, Nvim).\n\
                  2. Sets up the user configuration file (config.toml symlink).\n\
                  3. Provides instructions for shell integration.\n\
                  4. Configures Neovim (init.lua symlink, Packer install/sync)."
)]
pub struct AllArgs {
    /// Force Neovim setup steps: re-symlink init.lua (backing up existing),
    /// re-clone Packer if exists, re-run PackerSync.
    /// This flag is passed down directly to the `nvim` handler.
    #[arg(long, short = 'f')] // Allow -f for force
    force_nvim: bool,

    /// Skip the Neovim Packer plugin installation step (PackerSync).
    /// Packer itself will still be installed/checked.
    /// This flag is passed down directly to the `nvim` handler.
    #[arg(long)]
    skip_nvim_plugins: bool,
}

/// # Handle All Setup Command (`handle_all`)
///
/// Asynchronous handler function that orchestrates the complete DevRS host setup process
/// by calling the specific handlers for dependencies, config, shell integration, and nvim in order.
///
/// ## Arguments
///
/// * `args`: The parsed `AllArgs` struct containing flags that modify the Neovim setup behavior.
///
/// ## Returns
///
/// * `Result<()>`: `Ok(())` if all setup steps complete successfully.
/// * `Err`: If any setup step's handler function returns an error, the error is propagated immediately,
///   and subsequent steps are not executed.
pub async fn handle_all(args: AllArgs) -> Result<()> {
    info!("Handling setup all command...");
    println!("Starting full DevRS host setup...");

    // --- Step 1: Check Dependencies ---
    println!("\n--- Step 1: Checking Dependencies ---");
    // Call the handler from the 'deps' module.
    deps::handle_deps(deps::DepsArgs::default())
        .await // Await the async dependency check.
        .context("Dependency check failed. Please install missing tools manually.")?; // Add context if it fails.
    println!("-----------------------------------");

    // --- Step 2: Setup config.toml ---
    println!("\n--- Step 2: Setting up config.toml ---");
    // Call the handler from the new 'config' module.
    config::handle_config(config::ConfigArgs::default()) // Pass default (empty) args.
        .await // Await the async config setup.
        .context("Failed to set up config.toml")?; // Add context if it fails.
    println!("-----------------------------------");

    // --- Step 3: Provide Shell Integration Instructions ---
    println!("\n--- Step 3: Providing Shell Integration Instructions ---");
    // Call the handler from the 'integrate' module.
    integrate::handle_integrate(integrate::IntegrateArgs::default())
        .await // Await the async handler.
        .context("Failed to provide shell integration instructions.")?; // Add context if it fails.
    println!("-----------------------------------");

    // --- Step 4: Setup Nvim ---
    println!("\n--- Step 4: Configuring Neovim ---");
    // Create the specific arguments needed for the nvim handler, passing through
    // the relevant flags provided to the 'all' command.
    let nvim_args = nvim::NvimArgs {
        force: args.force_nvim,
        skip_plugins: args.skip_nvim_plugins,
    };
    // Call the handler from the 'nvim' module with the constructed arguments.
    nvim::handle_nvim(nvim_args)
        .await // Await the async Neovim setup.
        .context("Neovim setup failed.")?; // Add context if it fails.
    println!("-----------------------------------");

    // --- Completion Message ---
    println!("\n✅ DevRS host setup tasks finished!");
    // Remind user about the manual step required from Step 3 (shell integration).
    println!(
        "IMPORTANT: Please ensure you manually add the 'source' line from Step 3 to your shell config"
    );
    println!("           and then reload your shell or open a new terminal window.");

    Ok(()) // Indicate overall success of the 'all' sequence.
}


// --- Unit Tests --- (Keep existing tests for AllArgs parsing)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_args_parsing() {
        let args_default = AllArgs::try_parse_from(&["all"]).unwrap();
        assert!(!args_default.force_nvim);
        assert!(!args_default.skip_nvim_plugins);

        let args_flags = AllArgs::try_parse_from(&[
            "all",
            "--force-nvim",
            "--skip-nvim-plugins",
        ])
        .unwrap();
        assert!(args_flags.force_nvim);
        assert!(args_flags.skip_nvim_plugins);

        let args_short_force = AllArgs::try_parse_from(&["all", "-f"]).unwrap();
        assert!(args_short_force.force_nvim);
        assert!(!args_short_force.skip_nvim_plugins);
    }

    #[tokio::test]
    #[ignore] // Requires extensive mocking of handlers.
    async fn test_handle_all_orchestration() {
        // --- Mocking Setup (Conceptual) ---
        // Mock deps::handle_deps -> Ok(())
        // Mock config::handle_config -> Ok(()) // Mock the NEW handler
        // Mock integrate::handle_integrate -> Ok(())
        // Mock nvim::handle_nvim -> Ok(())

        // --- Test Execution ---
        let args = AllArgs {
            force_nvim: true,
            skip_nvim_plugins: true,
        };
        let result = handle_all(args).await;

        // --- Assertions ---
        assert!(result.is_ok());
        // Verify mocks show that deps, config, integrate, nvim handlers
        // were all called in the correct order.
        // Verify that nvim::handle_nvim received the correct arguments.
    }
}
