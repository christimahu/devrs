//! # DevRS Environment Command Group
//!
//! File: cli/src/commands/env/mod.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module serves as the entry point and router for the `devrs env`
//! command group. This group is responsible for managing the **core development
//! environment container**, which is a single, persistent Docker container providing
//! a standardized set of development tools used across projects. This module defines
//! the available subcommands (`build`, `shell`, `exec`, `status`, etc.) and delegates
//! execution to the appropriate submodule handlers.
//!
//! ## Architecture
//!
//! The module uses Clap's derive macros to define the command structure:
//! - `EnvArgs`: Top-level arguments struct for the `devrs env` group.
//! - `EnvCommand`: Enum defining all available environment subcommands.
//! - `handle_env`: The main handler function that matches the subcommand and routes
//!   execution to the corresponding handler in the submodules.
//!
//! Each specific subcommand's logic resides in its own `.rs` file within this
//! directory (e.g., `build.rs`, `shell.rs`).
//!
//! ## Examples
//!
//! Usage examples:
//!
//! ```bash
//! # Build the core environment image
//! devrs env build
//!
//! # Start an interactive shell within the core environment
//! devrs env shell
//!
//! # Execute a command directly within the core environment
//! devrs env exec cargo check
//!
//! # Check the status of the core environment container
//! devrs env status
//!
//! # Stop the core environment container
//! devrs env stop
//! ```
//!
//! The command processing flow:
//! 1. Parse `devrs env <subcommand> [args...]` using Clap.
//! 2. The `handle_env` function matches on the `<subcommand>`.
//! 3. The corresponding handler function (e.g., `build::handle_build`) is called.
//! 4. Results or errors are returned up the call stack.
//!
use crate::core::error::Result; // Use the standard Result type for error handling.
use clap::{Parser, Subcommand}; // Import necessary components from the Clap crate for argument parsing.

// --- Subcommand Module Declarations ---
// Declare the modules containing the implementation for each subcommand within the 'env' group.
// This makes the code within each corresponding `.rs` file available under its namespace (e.g., `build::`).

/// Implements the `devrs env build` command (builds the core env image).
mod build;
/// Implements the `devrs env exec` command (executes commands in the core env).
mod exec;
/// Implements the `devrs env logs` command (views logs from the core env container).
mod logs;
/// Implements the `devrs env prune` command (cleans up unused core env resources).
mod prune;
/// Implements the `devrs env rebuild` command (stops, removes, and rebuilds the core env).
mod rebuild;
/// Implements the `devrs env shell` command (starts an interactive shell in the core env).
mod shell;
/// Implements the `devrs env status` command (shows the status of the core env container).
mod status;
/// Implements the `devrs env stop` command (stops the core env container).
mod stop;

/// # Environment Command Group Arguments (`EnvArgs`)
///
/// This struct represents the top-level command group `devrs env`.
/// It utilizes `clap::Parser` to define the structure for arguments passed after `devrs env`.
/// Its primary role is to capture which specific subcommand (build, shell, exec, etc.)
/// the user intends to execute via the `command` field.
#[derive(Parser, Debug)]
pub struct EnvArgs {
    /// The specific environment subcommand to execute.
    /// The `#[command(subcommand)]` attribute signals to Clap that this field
    /// expects one of the variants defined in the `EnvCommand` enum below.
    #[command(subcommand)]
    command: EnvCommand,
}

/// # Environment Subcommands (`EnvCommand`)
///
/// This enum enumerates all valid subcommands available under `devrs env`.
/// It uses `clap::Subcommand` to enable parsing of these specific actions related
/// to the core development environment container.
///
/// Each variant corresponds to a distinct operation on the core environment and holds
/// the specific arguments struct defined within that subcommand's corresponding module.
/// This allows Clap to parse arguments relevant only to the chosen action (e.g.,
/// `--no-cache` for `build`, `--name` for `exec`, etc.).
#[derive(Subcommand, Debug)]
enum EnvCommand {
    /// Corresponds to `devrs env build`. Builds the core development environment Docker image.
    /// Holds `build::BuildArgs` for options like `--no-cache`, `--stage`.
    Build(build::BuildArgs),
    /// Corresponds to `devrs env exec`. Executes a specified command inside the running core environment container.
    /// Holds `exec::ExecArgs` for options like `-i`, `-t`, `--user`, `--workdir`, and the command itself.
    Exec(exec::ExecArgs),
    /// Corresponds to `devrs env logs`. Fetches and displays logs from the core environment container.
    /// Holds `logs::LogsArgs` for options like `--follow`, `--lines`, `--name`.
    Logs(logs::LogsArgs),
    /// Corresponds to `devrs env prune`. Removes stopped core environment containers.
    /// Holds `prune::PruneArgs` for the `--force` flag.
    Prune(prune::PruneArgs),
    /// Corresponds to `devrs env rebuild`. Stops, removes, and then rebuilds the core environment image and container.
    /// Holds `rebuild::RebuildArgs` for options like `--no-cache`, `--name`, `--with-plugins`.
    Rebuild(rebuild::RebuildArgs),
    /// Corresponds to `devrs env shell`. Starts an interactive shell session within the running core environment container.
    /// Holds `shell::ShellArgs` for the optional `--name` override.
    Shell(shell::ShellArgs),
    /// Corresponds to `devrs env status`. Displays detailed status information about the core environment container.
    /// Holds `status::StatusArgs` for the optional `--name` override.
    Status(status::StatusArgs),
    /// Corresponds to `devrs env stop`. Stops the running core environment container.
    /// Holds `stop::StopArgs` for options like `--name` and `--time`.
    Stop(stop::StopArgs),
}

/// # Handle Environment Command (`handle_env`)
///
/// The main entry point function for the `devrs env` command group.
/// It acts as a **dispatcher**, receiving the parsed arguments (`EnvArgs`)
/// which contain the specific subcommand chosen by the user.
///
/// It uses a `match` statement to determine which subcommand variant was provided
/// (e.g., `Build`, `Shell`, `Exec`) and calls the corresponding asynchronous handler
/// function (e.g., `build::handle_build`, `shell::handle_shell`) from the relevant submodule,
/// passing along the arguments specific to that subcommand.
///
/// ## Arguments
///
/// * `args`: The parsed `EnvArgs` struct containing the specific `EnvCommand` variant and its associated arguments.
///
/// ## Returns
///
/// * `Result<()>`: Propagates the `Result` from the called subcommand handler. Returns `Ok(())`
///   if the subcommand executed successfully, or an `Err` if the subcommand handler encountered an error.
pub async fn handle_env(args: EnvArgs) -> Result<()> {
    // Match on the specific subcommand variant provided in the parsed arguments.
    match args.command {
        // Route execution based on the matched command.
        EnvCommand::Build(args) => build::handle_build(args).await?, // Call build handler.
        EnvCommand::Exec(args) => exec::handle_exec(args).await?,    // Call exec handler.
        EnvCommand::Logs(args) => logs::handle_logs(args).await?,    // Call logs handler.
        EnvCommand::Prune(args) => prune::handle_prune(args).await?, // Call prune handler.
        EnvCommand::Rebuild(args) => rebuild::handle_rebuild(args).await?, // Call rebuild handler.
        EnvCommand::Shell(args) => shell::handle_shell(args).await?, // Call shell handler.
        EnvCommand::Status(args) => status::handle_status(args).await?, // Call status handler.
        EnvCommand::Stop(args) => stop::handle_stop(args).await?,    // Call stop handler.
    }
    // If the matched handler completed successfully, return Ok.
    Ok(())
}

// --- Unit Tests ---
// This test is a basic placeholder ensuring the module structure compiles.
// More specific tests for argument parsing and potentially routing (using mocks)
// would typically reside here or be invoked from the main integration tests.
#[cfg(test)]
mod tests {
    // Basic test for the module structure compilation.
    #[test]
    fn placeholder_env_mod_test() {
        // This test doesn't perform any actions but confirms that the module
        // definition and its dependencies compile correctly.
        // Actual parsing tests similar to those in `commands/container/mod.rs`
        // could be added here to verify Clap parsing for `devrs env` subcommands.
        assert!(true); // Placeholder assertion.
    }
}
