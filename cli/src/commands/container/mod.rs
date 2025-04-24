//! # DevRS Container Command Group
//!
//! File: cli/src/commands/container/mod.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module serves as the entry point and router for the `devrs container`
//! command group. It defines the available subcommands (like `build`, `run`, `logs`, `rm`, etc.)
//! related to managing application-specific Docker containers and delegates execution
//! to the appropriate submodule handlers.
//!
//! ## Architecture
//!
//! The module uses Clap's derive macros to define the command structure:
//! - `ContainerArgs`: Top-level arguments struct for the `devrs container` group.
//! - `ContainerCommand`: Enum defining all available container subcommands.
//! - `handle_container`: The main handler function that matches the subcommand and routes
//!   execution to the corresponding handler in the submodules.
//!
//! Each specific subcommand's logic resides in its own `.rs` file within this directory
//! (e.g., `build.rs`, `run.rs`).
//!
//! ## Examples
//!
//! Usage examples:
//!
//! ```bash
//! # Build a container image from the current directory
//! devrs container build --tag myapp:1.0
//!
//! # Run a container from an image
//! devrs container run --image myapp:1.0 --port 8080:80
//!
//! # View logs from a running container
//! devrs container logs my-container
//!
//! # Remove a container
//! devrs container rm my-container
//! ```
//!
//! The command processing flow:
//! 1. Parse `devrs container <subcommand> [args...]` using Clap.
//! 2. The `handle_container` function matches on the `<subcommand>`.
//! 3. The corresponding handler function (e.g., `build::handle_build`) is called.
//! 4. Results or errors are returned.
//!
use crate::core::error::Result; // Use the standard Result type for error handling.
use clap::{Parser, Subcommand}; // Import necessary components from the Clap crate for argument parsing.

// --- Subcommand Module Declarations ---
// Declare the modules that contain the implementation for each subcommand.
// This makes the code within `build.rs`, `run.rs`, etc., available under
// their respective namespaces (e.g., `build::`, `run::`) within this file.

/// Implements the `devrs container build` command.
mod build;
/// Implements the `devrs container logs` command.
mod logs;
/// Implements the `devrs container rm` command (remove containers).
mod rm;
/// Implements the `devrs container rmi` command (remove images).
mod rmi;
/// Implements the `devrs container run` command.
mod run;
/// Implements the `devrs container shell` command (debug shell).
mod shell;
/// Implements the `devrs container status` command.
mod status;
/// Implements the `devrs container stop` command.
mod stop;

/// # Container Command Group Arguments (`ContainerArgs`)
///
/// This struct represents the top-level command group `devrs container`.
/// It utilizes `clap::Parser` to define the structure for arguments passed
/// after `devrs container`. Its main purpose is to capture which specific
/// subcommand (build, run, logs, etc.) the user intends to execute.
#[derive(Parser, Debug)]
pub struct ContainerArgs {
    /// The specific container subcommand to execute (e.g., build, run, rm).
    /// The `#[command(subcommand)]` attribute signals to Clap that this field
    /// expects one of the variants defined in the `ContainerCommand` enum below.
    #[command(subcommand)]
    command: ContainerCommand,
}

/// # Container Subcommands (`ContainerCommand`)
///
/// This enum enumerates all valid subcommands available under `devrs container`.
/// It uses `clap::Subcommand` to enable parsing of these specific actions.
///
/// Each variant corresponds to a distinct container-related operation and is designed
/// to hold the specific arguments struct defined within that subcommand's module.
/// For instance, the `Build` variant holds `build::BuildArgs`, allowing Clap to parse
/// arguments like `--tag` or `--file` only when the `build` subcommand is used.
#[derive(Subcommand, Debug)]
enum ContainerCommand {
    /// Corresponds to `devrs container build`.
    /// Builds an application Docker image, typically from the current directory's Dockerfile.
    /// Holds `build::BuildArgs` for options like `--tag`, `--file`, `--no-cache`.
    Build(build::BuildArgs), //

    // Note: The `Buildrun` subcommand was previously present but has been removed.
    // Users should now use separate `build` and `run` commands.

    /// Corresponds to `devrs container logs`.
    /// Fetches and displays logs from a specified running or stopped application container.
    /// Holds `logs::LogsArgs` for options like container name/ID, `--follow`, `--lines`.
    Logs(logs::LogsArgs), //

    /// Corresponds to `devrs container rm`.
    /// Removes one or more specified application containers.
    /// Holds `rm::RmArgs` for options like container names/IDs and `--force`.
    Rm(rm::RmArgs), //

    /// Corresponds to `devrs container rmi`.
    /// Removes one or more specified application Docker images.
    /// Holds `rmi::RmiArgs` for options like image names/IDs and `--force`.
    Rmi(rmi::RmiArgs), //

    /// Corresponds to `devrs container run`.
    /// Creates and starts a new application container from a specified image.
    /// Holds `run::RunArgs` for options like `--image`, `--name`, `--port`, `--env`, `--detach`, `--rm`, and command overrides.
    Run(run::RunArgs), //

    /// Corresponds to `devrs container shell`.
    /// Starts a temporary, interactive shell within a specified application image, mainly for debugging purposes. The container is auto-removed on exit.
    /// Holds `shell::ShellArgs` for options like image name and the command/shell to run.
    Shell(shell::ShellArgs), //

    /// Corresponds to `devrs container status`.
    /// Displays the status of running or all application containers (excluding the core DevRS environment container).
    /// Holds `status::StatusArgs` for the `--all` flag.
    Status(status::StatusArgs), //

    /// Corresponds to `devrs container stop`.
    /// Stops one or more specified running application containers.
    /// Holds `stop::StopArgs` for options like container names/IDs and `--time`.
    Stop(stop::StopArgs), //
}

/// # Handle Container Command (`handle_container`)
///
/// The main entry point function for the `devrs container` command group.
/// It acts as a **dispatcher**, receiving the parsed arguments (`ContainerArgs`)
/// which contain the specific subcommand chosen by the user.
///
/// It uses a `match` statement to determine which subcommand variant was provided
/// (e.g., `Build`, `Run`, `Logs`) and calls the corresponding asynchronous handler
/// function (e.g., `build::handle_build`, `run::handle_run`) from the relevant submodule,
/// passing along the arguments specific to that subcommand.
///
/// ## Arguments
///
/// * `args`: The parsed `ContainerArgs` struct containing the specific `ContainerCommand` variant and its associated arguments.
///
/// ## Returns
///
/// * `Result<()>`: Propagates the `Result` from the called subcommand handler. Returns `Ok(())`
///   if the subcommand executed successfully, or an `Err` if the subcommand handler encountered an error.
pub async fn handle_container(args: ContainerArgs) -> Result<()> {
    // Match on the specific subcommand variant provided in the parsed arguments.
    match args.command {
        // If the command was `build`...
        ContainerCommand::Build(args) => {
            // ...call the `handle_build` function from the `build` module.
            build::handle_build(args).await? // `await` the async handler and propagate errors (`?`).
        }
        // Buildrun variant removed.
        // If the command was `logs`...
        ContainerCommand::Logs(args) => {
            // ...call the `handle_logs` function from the `logs` module.
            logs::handle_logs(args).await?
        }
        // If the command was `rm`...
        ContainerCommand::Rm(args) => {
            // ...call the `handle_rm` function from the `rm` module.
            rm::handle_rm(args).await?
        }
        // If the command was `rmi`...
        ContainerCommand::Rmi(args) => {
            // ...call the `handle_rmi` function from the `rmi` module.
            rmi::handle_rmi(args).await?
        }
        // If the command was `run`...
        ContainerCommand::Run(args) => {
            // ...call the `handle_run` function from the `run` module.
            run::handle_run(args).await?
        }
        // If the command was `shell`...
        ContainerCommand::Shell(args) => {
            // ...call the `handle_shell` function from the `shell` module.
            shell::handle_shell(args).await?
        }
        // If the command was `status`...
        ContainerCommand::Status(args) => {
            // ...call the `handle_status` function from the `status` module.
            status::handle_status(args).await?
        }
        // If the command was `stop`...
        ContainerCommand::Stop(args) => {
            // ...call the `handle_stop` function from the `stop` module.
            stop::handle_stop(args).await?
        }
    }
    // If the matched handler successfully completed (returned Ok), return Ok(()) from this dispatcher.
    Ok(())
}


// --- Unit Tests ---
// These tests primarily verify that `clap` correctly parses the command-line arguments
// for the `container` command group and its various subcommands. They ensure that
// the defined argument structures and subcommand enum work as intended.
// Testing the actual routing logic within `handle_container` typically requires mocking.
#[cfg(test)]
mod tests {
    use super::*; // Import items from the parent module (container/mod.rs).

    /// Test parsing of the `build` subcommand and its specific arguments.
    #[test]
    fn test_parses_container_build() {
        // Simulate `devrs container build --file Dockerfile.prod`
        // Note: The first "container" is for context, telling clap which top-level command we're testing.
        let result =
            ContainerArgs::try_parse_from(&["container", "build", "--file", "Dockerfile.prod"]);
        assert!(result.is_ok()); // Check if parsing succeeded.
        // Verify the correct subcommand variant was matched.
        match result.unwrap().command {
            ContainerCommand::Build(_) => {} // Expected variant.
            _ => panic!("Incorrect subcommand parsed for 'build'"), // Fail if wrong variant.
        }
    }

    /// Test parsing of the `run` subcommand and its specific arguments.
    #[test]
    fn test_parses_container_run() {
        // Simulate `devrs container run --image test`
        let result = ContainerArgs::try_parse_from(&["container", "run", "--image", "test"]);
        assert!(result.is_ok());
        match result.unwrap().command {
            ContainerCommand::Run(_) => {} // Expected variant.
            _ => panic!("Incorrect subcommand parsed for 'run'"),
        }
    }

    /// Test parsing of the `rm` subcommand and its specific arguments.
    #[test]
    fn test_parses_container_rm() {
        // Simulate `devrs container rm my-container`
        let result = ContainerArgs::try_parse_from(&["container", "rm", "my-container"]);
        assert!(result.is_ok());
        match result.unwrap().command {
            ContainerCommand::Rm(_) => {} // Expected variant.
            _ => panic!("Incorrect subcommand parsed for 'rm'"),
        }
    }

    /// Test that the previously removed 'buildrun' subcommand is correctly rejected.
    #[test]
    fn test_rejects_container_buildrun() {
        // Simulate `devrs container buildrun --tag test`
        let result = ContainerArgs::try_parse_from(&["container", "buildrun", "--tag", "test"]);
        // Expect parsing to fail because `Buildrun` is no longer a variant in `ContainerCommand`.
        assert!(
            result.is_err(),
            "'buildrun' should no longer be a valid subcommand"
        );
    }

    // TODO: Add similar parsing tests for the other subcommands:
    // - logs
    // - rmi
    // - shell
    // - status
    // - stop
    // These would follow the same pattern as the tests above, ensuring clap
    // recognizes the subcommand and potentially some of its key arguments.
}
