//! # DevRS Blueprint Command Group
//!
//! File: cli/src/commands/blueprint/mod.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module serves as the entry point and router for the `devrs blueprint`
//! command group. It defines the available subcommands (`list`, `create`, `info`)
//! related to managing project templates (blueprints) and delegates the execution
//! to the appropriate submodule handlers.
//!
//! ## Architecture
//!
//! The module uses Clap's derive macros to define the command structure:
//! - `BlueprintArgs`: Top-level arguments for the command group.
//! - `BlueprintCommand`: Enum defining all blueprint subcommands.
//! - `handle_blueprint`: Main handler function that routes execution to the relevant subcommand handler.
//!
//! Each specific subcommand's logic is implemented in its own corresponding `.rs` file
//! within this directory (e.g., `create.rs`, `info.rs`, `list.rs`). Shared utilities
//! specific to blueprints reside in the `utils` submodule.
//!
//! ## Examples
//!
//! Usage examples:
//!
//! ```bash
//! # List available blueprints
//! devrs blueprint list
//!
//! # Create a new project from a blueprint
//! devrs blueprint create --lang rust my-awesome-project
//!
//! # Get detailed info about a blueprint
//! devrs blueprint info rust
//! ```
//!
//! The command routing flow:
//! 1. Parse `devrs blueprint <subcommand> [args...]` using Clap.
//! 2. The `handle_blueprint` function matches on the `<subcommand>`.
//! 3. The corresponding handler function (e.g., `list::handle_list`) is called with its specific arguments.
//! 4. Results (or errors) are returned up the call stack.
//!
use crate::core::error::Result; // Use the standard Result type for error handling.
use clap::{Parser, Subcommand}; // Import necessary components from the Clap crate for argument parsing.

// --- Subcommand Module Declarations ---
// Declare the modules that contain the implementation for each subcommand.
// This makes the code within `create.rs`, `info.rs`, and `list.rs` available under
// the `create::`, `info::`, and `list::` namespaces respectively within this file.

/// Contains the handler and arguments for the `devrs blueprint create` subcommand.
mod create;
/// Contains the handler and arguments for the `devrs blueprint info` subcommand.
mod info;
/// Contains the handler and arguments for the `devrs blueprint list` subcommand.
mod list;
/// Contains shared utility functions specific to the blueprint commands (e.g., project detection, tree printing).
/// Declared as `pub` so it might be potentially accessible by other command modules if needed,
/// though primarily intended for use within the `blueprint` subcommands.
pub mod utils; //

/// # Blueprint Command Group Arguments (`BlueprintArgs`)
///
/// This struct represents the top-level command group `devrs blueprint`.
/// It uses `clap::Parser` to define how arguments for this group are structured.
/// Its primary role is to capture which specific subcommand (`list`, `create`, or `info`)
/// the user wants to execute.
#[derive(Parser, Debug)]
pub struct BlueprintArgs {
    /// The specific blueprint subcommand to execute (e.g., list, create, info).
    /// The `#[command(subcommand)]` attribute tells Clap to expect one of the
    /// variants defined in the `BlueprintCommand` enum as the next argument.
    #[command(subcommand)]
    command: BlueprintCommand,
}

/// # Blueprint Subcommands (`BlueprintCommand`)
///
/// This enum defines the set of valid subcommands that can follow `devrs blueprint`.
/// Each variant corresponds to a specific action related to blueprints.
/// Using `clap::Subcommand` allows `clap` to parse which action the user intends.
///
/// Each variant holds the specific arguments struct required by that subcommand's handler,
/// allowing `clap` to parse further arguments relevant only to that specific action.
#[derive(Subcommand, Debug)]
enum BlueprintCommand {
    /// Corresponds to `devrs blueprint list`.
    /// Lists available project blueprints found in the configured directory.
    /// Holds `list::ListArgs` (which is currently an empty struct).
    List(list::ListArgs), //
    /// Corresponds to `devrs blueprint create`.
    /// Creates a new project directory by rendering a specified blueprint template.
    /// Holds `create::CreateArgs`, containing options like `--lang`, `--output`, `--var`, etc.
    Create(create::CreateArgs), //
    /// Corresponds to `devrs blueprint info`.
    /// Shows detailed information about a specific blueprint template.
    /// Holds `info::InfoArgs`, containing the name of the blueprint to inspect.
    Info(info::InfoArgs), //
}

/// # Handle Blueprint Command (`handle_blueprint`)
///
/// The main entry point function for the `devrs blueprint` command group.
/// It receives the parsed arguments (`BlueprintArgs`) which include the specific
/// subcommand chosen by the user.
///
/// Its primary responsibility is to act as a **dispatcher**: it matches on the
/// `command` enum variant within `args` and calls the appropriate asynchronous
/// handler function (e.g., `list::handle_list`, `create::handle_create`)
/// associated with that subcommand, passing along the subcommand-specific arguments.
///
/// ## Arguments
///
/// * `args`: The parsed `BlueprintArgs` struct containing the specific `BlueprintCommand` variant and its associated arguments.
///
/// ## Returns
///
/// * `Result<()>`: Propagates the `Result` from the called subcommand handler. Returns `Ok(())`
///   if the subcommand executed successfully, or an `Err` if the subcommand handler encountered an error.
pub async fn handle_blueprint(args: BlueprintArgs) -> Result<()> {
    // Match on the subcommand specified by the user.
    match args.command {
        // If the command was `list`...
        BlueprintCommand::List(args) => {
            // ...call the handler function from the `list` module, passing its specific arguments.
            list::handle_list(args).await? // `await` the async handler and propagate errors with `?`.
        }
        // If the command was `create`...
        BlueprintCommand::Create(args) => {
            // ...call the handler function from the `create` module.
            create::handle_create(args).await?
        }
        // If the command was `info`...
        BlueprintCommand::Info(args) => {
            // ...call the handler function from the `info` module.
            info::handle_info(args).await?
        }
    }
    // If the matched handler completed successfully (didn't return Err), return Ok.
    Ok(())
}


// --- Unit Tests ---
// These tests focus on ensuring that the argument parsing for the `blueprint`
// command group and its subcommands works as expected via Clap. They also
// include a placeholder/ignored test structure for verifying command routing,
// which would typically require mocking in a full test suite.
#[cfg(test)]
mod tests {
    use super::*; // Import items from the parent module (blueprint/mod.rs)
    use crate::core::config; // Required for the mock config helper type signature
    use std::{fs, path::PathBuf}; // Required for test setup helpers
    use tempfile::tempdir; // For creating temporary directories in tests

    /// Test that `clap` correctly parses the `devrs blueprint list` command.
    #[test]
    fn test_parses_blueprint_list() {
        // Simulate command-line input: `devrs blueprint list`
        let result = BlueprintArgs::try_parse_from(["blueprint", "list"]);
        // Expect parsing to succeed.
        assert!(result.is_ok());
        // Verify that the parsed command is indeed the `List` variant.
        match result.unwrap().command {
            BlueprintCommand::List(_) => {} // Correct variant found.
            _ => panic!("Incorrect subcommand parsed for 'list'"), // Fail if wrong variant parsed.
        }
    }

    /// Test that `clap` correctly parses the `devrs blueprint create` command and its arguments.
    #[test]
    fn test_parses_blueprint_create() {
         // Simulate command-line input: `devrs blueprint create --lang rust my-project`
        let result =
            BlueprintArgs::try_parse_from(["blueprint", "create", "--lang", "rust", "my-project"]);
        // Expect parsing to succeed.
        assert!(result.is_ok());
        // Verify that the parsed command is the `Create` variant.
        match result.unwrap().command {
            BlueprintCommand::Create(_) => {} // Correct variant found.
            _ => panic!("Incorrect subcommand parsed for 'create'"), // Fail if wrong variant parsed.
        }
    }

    /// Test that `clap` correctly parses the `devrs blueprint info` command and its arguments.
    #[test]
    fn test_parses_blueprint_info() {
        // Simulate command-line input: `devrs blueprint info rust`
        let result = BlueprintArgs::try_parse_from(["blueprint", "info", "rust"]);
        // Expect parsing to succeed.
        assert!(result.is_ok());
         // Verify that the parsed command is the `Info` variant.
        match result.unwrap().command {
            BlueprintCommand::Info(_) => {} // Correct variant found.
            _ => panic!("Incorrect subcommand parsed for 'info'"), // Fail if wrong variant parsed.
        }
    }

    // Helper function to create a mock config for testing handlers.
    // This would ideally use a proper mocking library or dependency injection
    // in a real application, but serves as a concept here.
    #[allow(dead_code)] // Allow this helper to be unused for now.
    fn mock_config_loading(bp_dir: PathBuf) -> config::Config {
        // Create a default config instance.
        let mut cfg = config::Config::default();
        // Override the blueprint directory path with the provided temporary path.
        cfg.blueprints.directory = bp_dir.to_string_lossy().to_string();
        cfg // Return the modified config.
    }

    /// Test that the `handle_blueprint` function routes commands correctly.
    /// Note: This test is ignored because properly testing the routing without
    /// executing the actual (potentially unimplemented or complex) handlers
    /// requires mocking the handlers themselves or the functions they call (like config loading).
    #[tokio::test]
    #[ignore] // Ignoring test because mocking `config::load_config` and handlers is non-trivial here.
    async fn test_handle_blueprint_routing() {
        // --- Test Setup ---
        // 1. Create a temporary directory to act as the blueprint root.
        let temp_dir = tempdir().unwrap();
        let bp_path = temp_dir.path().to_path_buf();
        // 2. Create a dummy blueprint subdirectory required by some commands (like 'info' or 'create').
        fs::create_dir(bp_path.join("rust")).unwrap();
        // 3. (Ideally) Mock the `config::load_config` function to return our mock config.
        //    This step is complex without a mocking library and is omitted here.

        // --- Test list command routing (conceptual) ---
        let list_args = BlueprintArgs::try_parse_from(["blueprint", "list"]).unwrap();
        // Expect handle_blueprint to call list::handle_list.
        // Without mocking, this will likely fail due to the real config loading.
        assert!(
            handle_blueprint(list_args).await.is_err(),
            "List handler expects config loading to work"
        );

        // --- Test info command routing (conceptual) ---
        let info_args = BlueprintArgs::try_parse_from(["blueprint", "info", "rust"]).unwrap();
         // Expect handle_blueprint to call info::handle_info.
        assert!(
            handle_blueprint(info_args).await.is_err(),
            "Info handler expects config loading to work"
        );

        // --- Test create command routing (conceptual) ---
        let create_args = BlueprintArgs::try_parse_from([
            "blueprint",
            "create",
            "--lang",
            "rust",
            "my-test-proj",
            "--output", // Use the temp dir as output to avoid permission issues
            temp_dir.path().to_str().unwrap(),
        ])
        .unwrap();
         // Expect handle_blueprint to call create::handle_create.
        assert!(
            handle_blueprint(create_args).await.is_err(),
            "Create handler expects config and templating"
        );

        // --- Cleanup (Conceptual) ---
        // If mocking was used, restore original functions here.
    }
}
