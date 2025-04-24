//! # DevRS Environment Execution Handler
//!
//! File: cli/src/commands/env/exec.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs env exec` subcommand. Its purpose is to
//! execute a specified command *inside* the running core development environment
//! container without needing to start a full interactive shell session first
//! (unlike `devrs env shell`). This is useful for running quick commands, scripts,
//! or build tasks directly within the configured environment.
//!
//! ## Architecture
//!
//! The command flow is as follows:
//! 1. Parse command-line arguments (`ExecArgs`) using `clap`, capturing flags like `-i` (interactive), `-t` (TTY), `--name`, `--user`, `--workdir`, and the command itself.
//! 2. Load the DevRS configuration (`core::config`) to determine the core environment settings (like image name to derive the default container name).
//! 3. Determine the target core environment container name (using `--name` if provided, otherwise generating the default `<image_name>-instance`).
//! 4. Call the shared `common::docker::lifecycle::ensure_core_env_running` utility function to guarantee the target container exists and is in a running state, starting or creating it if necessary.
//! 5. Validate that the user provided a command to execute.
//! 6. Call the shared `common::docker::interaction::exec_in_container` utility function, passing the container name, command, and all relevant flags (`interactive`, `tty`, `user`, `workdir`). This function handles the Docker `exec` API call and manages I/O streaming.
//! 7. Check the exit code returned by `exec_in_container`. If the exit code is 0, return `Ok(())`. If it's non-zero, return an `Err` of type `DevrsError::ExternalCommand` containing the command and exit code.
//!
//! ## Usage
//!
//! ```bash
//! # Run a simple, non-interactive command
//! devrs env exec ls -la /home/me/code
//!
//! # Run a build command
//! devrs env exec cargo build --release
//!
//! # Run an interactive command needing TTY (e.g., a tool with interactive prompts)
//! devrs env exec -it some_tool --configure
//!
//! # Run a command as a specific user inside the container
//! devrs env exec --user root apt update
//!
//! # Run a command in a specific working directory inside the container
//! devrs env exec -w /home/me/code/my-project git status
//!
//! # Run in a specifically named core environment container
//! devrs env exec --name my-custom-env-instance make all
//! ```
//!
//! The `-i` and `-t` flags mimic the behavior of `docker exec -it`, enabling interactive sessions when needed.
//!
use crate::{
    common::docker::{self}, // Access shared Docker utilities (ensure_running, exec_in_container).
    core::{
        config, // Access configuration loading.
        error::{DevrsError, Result}, // Standard Result type and custom errors.
    },
};
use anyhow::{anyhow, Context}; // For easy error creation and adding context.
use clap::Parser; // For parsing command-line arguments.
use tracing::{debug, info, warn}; // Logging framework utilities.

/// # Environment Execute Arguments (`ExecArgs`)
///
/// Defines the command-line arguments accepted by the `devrs env exec` subcommand.
/// Uses the `clap` crate for parsing and validation.
#[derive(Parser, Debug)]
#[command(
    about = "Execute a command in the core development environment container",
    long_about = "Ensures the core environment container is running and executes the specified command inside it."
)]
pub struct ExecArgs {
    /// Optional: Runs the command interactively by allocating a pseudo-TTY (pseudo-terminal).
    /// Often used with `-t`[48;45;171;1980;3420t/`--tty`. Mimics `docker exec -i`.
    #[arg(short, long)] // Define as `-i` or `--interactive`.
    interactive: bool,

    /// Optional: Keeps STDIN open even if not attaching to a TTY.
    /// Typically used in conjunction with `-i`/`--interactive` for interactive commands.
    /// Mimics `docker exec -t`.
    #[arg(short = 't', long)] // Define as `-t` or `--tty`. Note: Clap uses `-t` for `tty`.
    tty: bool,

    /// Optional: Specifies the exact name of the core environment container to execute the command in.
    /// If omitted, the default name (`<core_env.image_name>-instance`) derived from the configuration is used.
    #[arg(long)] // Define as `--name <NAME>`.
    name: Option<String>,

    /// Optional: Specifies the username or UID to run the command as inside the container
    /// (e.g., 'root', '1000', 'vscode').
    /// If omitted, the command runs as the container's default configured user.
    #[arg(long, short)] // Define as `--user` or `-u`.
    user: Option<String>,

    /// Optional: Sets the working directory inside the container for the executed command.
    /// If omitted, uses the container's default working directory (often configured via `core_env.default_workdir`
    /// or the image's `WORKDIR` instruction).
    #[arg(long, short = 'w')] // Define as `--workdir` or `-w`.
    workdir: Option<String>,

    /// The command and its arguments to execute inside the core environment container.
    /// All arguments provided after the options (or after `--`) are captured into this vector.
    /// Example: `devrs env exec ls -la /tmp` -> `command` will be `vec!["ls", "-la", "/tmp"]`.
    #[arg(required = true, last = true)] // Mark as required, capture all remaining args.
    command: Vec<String>,
}

/// # Handle Environment Execute Command (`handle_exec`)
///
/// The main asynchronous handler function for the `devrs env exec` command.
/// It ensures the core development environment container is running and then executes
/// the specified command within it, handling interactivity flags and reporting the exit status.
///
/// ## Workflow:
/// 1.  Logs the start and parsed arguments.
/// 2.  Loads the DevRS configuration (`core::config`) to get core environment details.
/// 3.  Determines the target container name using `get_core_env_container_name` (uses `--name` override or generates default from config).
/// 4.  Calls `common::docker::lifecycle::ensure_core_env_running` to make sure the target container exists and is running, starting/creating it if necessary.
/// 5.  Validates that `args.command` is not empty (Clap's `required=true` should normally prevent this, but added check for safety).
/// 6.  Calls `common::docker::interaction::exec_in_container` with the container name, command vector, and the `interactive`, `tty`, `workdir`, and `user` arguments. This function handles the underlying Docker `exec` call and I/O streaming.
/// 7.  Checks the integer exit code returned by `exec_in_container`.
/// 8.  If the exit code is 0, logs success and returns `Ok(())`.
/// 9.  If the exit code is non-zero, logs a warning and returns an `Err` of type `DevrsError::ExternalCommand`, including the command string and exit code for clear error reporting.
///
/// ## Arguments
///
/// * `args`: The parsed `ExecArgs` struct containing all command-line options and the command to execute.
///
/// ## Returns
///
/// * `Result<()>`: Returns `Ok(())` if the command executes successfully within the container (exit code 0).
/// * `Err`: Returns an `Err` if config loading fails, the container cannot be prepared, no command was provided,
///   the Docker `exec` operation fails, or the executed command returns a non-zero exit code.
pub async fn handle_exec(args: ExecArgs) -> Result<()> {
    info!("Handling env exec command..."); // Log entry point.
    debug!("Exec args: {:?}", args); // Log parsed arguments.

    // 1. Load configuration.
    let cfg = config::load_config().context("Failed to load DevRS configuration")?;

    // 2. Determine the target core environment container name.
    let container_name = args.name.clone().unwrap_or_else(|| {
        // If --name was not provided, generate the default name.
        let default_name = get_core_env_container_name(&cfg);
        debug!("No specific name provided, using default: {}", default_name);
        default_name
    });

    // 3. Ensure the core environment container is ready (exists and is running).
    // This function will start or create the container if needed.
    let _ = docker::lifecycle::ensure_core_env_running(&container_name, &cfg) //
        .await // Await the async check/preparation.
        .with_context(|| format!("Failed to prepare container '{}' for exec", container_name))?;
    // We don't need the return value (bool indicating creation) here.

    // 4. Validate that a command was actually provided.
    // Clap's `required=true` should prevent this, but check defensively.
    if args.command.is_empty() {
        return Err(anyhow!(DevrsError::ArgumentParsing(
            "No command provided to execute.".to_string()
        )));
    }

    // 5. Execute the command inside the container using the shared utility.
    info!(
        "Executing command {:?} in container '{}' (Interactive: {}, TTY: {})",
        args.command, container_name, args.interactive, args.tty
    );
    let exit_code = docker::interaction::exec_in_container( //
        &container_name,        // Target container.
        &args.command,          // Command and arguments vector.
        args.interactive,       // Pass interactive flag.
        args.tty,               // Pass TTY flag.
        args.workdir.as_deref(), // Pass optional working directory.
        args.user.as_deref(),    // Pass optional user.
    )
    .await // Await the async execution.
    .with_context(|| { // Add context to potential errors during execution.
        format!(
            "Failed to execute command {:?} in container '{}'",
            args.command, container_name
        )
    })?;

    // 6. Check the exit code from the command execution.
    if exit_code == 0 {
        // Command succeeded (exit code 0).
        info!(
            "Command {:?} finished successfully (exit code 0) in container '{}'.",
            args.command, container_name
        );
        Ok(()) // Return Ok for overall success.
    } else {
        // Command failed (non-zero exit code).
        warn!(
            "Command {:?} finished with non-zero exit code: {} in container '{}'.",
            args.command, exit_code, container_name
        );
        // Return a specific error indicating external command failure.
        Err(anyhow!(DevrsError::ExternalCommand {
            // Include details in the error for better reporting.
            cmd: args.command.join(" "), // Reconstruct command string for error message.
            status: exit_code.to_string(), // Convert exit code to string.
            output: format!( // Provide context in the output field.
                "Command executed in container '{}' exited with code {}",
                container_name, exit_code
            ),
        }))
    }
}

/// # Get Core Environment Container Name (`get_core_env_container_name`)
///
/// A helper function to consistently derive the default name for the core development
/// environment container based on the image name defined in the configuration.
///
/// ## Logic:
/// Appends "-instance" to the configured `core_env.image_name`.
/// Example: If `core_env.image_name` is "devrs-core-tools", the container name will be "devrs-core-tools-instance".
///
/// ## Arguments
///
/// * `cfg`: A reference to the loaded `Config` struct.
///
/// ## Returns
///
/// * `String`: The derived container name.
fn get_core_env_container_name(cfg: &config::Config) -> String {
    // Use format! macro for simple string concatenation.
    format!("{}-instance", cfg.core_env.image_name)
}


// --- Unit Tests ---
// Focus on argument parsing for the `exec` command. Testing the handler logic
// requires mocking config loading and Docker interactions.
#[cfg(test)]
mod tests {
    use super::*; // Import items from parent module.

    /// Test parsing with various flags and a command with arguments.
    #[test]
    fn test_exec_args_parsing() {
        // Simulate `devrs env exec -it --user testuser -w /app ls -la`
        let args = ExecArgs::try_parse_from(&[
            "exec", // Command name context for clap.
            "-it",  // Combined interactive and tty flags.
            "--user", "testuser", // User flag.
            "-w", "/app", // Workdir flag.
            "--", // Add separator before trailing command args
            "ls",  // Start of the command to execute.
            "-la", // Argument for the command.            
        ])
        .unwrap(); // Expect parsing to succeed.

        // Verify flags.
        assert!(args.interactive); // -i was present.
        assert!(args.tty); // -t was present.
        // Verify optional arguments with values.
        assert_eq!(args.user, Some("testuser".to_string()));
        assert_eq!(args.workdir, Some("/app".to_string()));
        // Verify the captured command and its arguments.
        assert_eq!(args.command, vec!["ls", "-la"]);
        // Verify optional name argument is None by default.
        assert!(args.name.is_none());
    }

    /// Test parsing with the --name argument and a multi-part command.
    #[test]
    fn test_exec_args_parsing_with_name() {
        // Simulate `devrs env exec --name my-dev-container bash -c "echo hello"`
        let args = ExecArgs::try_parse_from(&[
            "exec",
            "--name", // Specify container name.
            "my-dev-container",
            "--", // Add separator before trailing command args
            "bash", // Command part 1.
            "-c", // Command part 2.
            "echo hello", // Command part 3 (treated as a single argument here).
        ])
        .unwrap();

        // Verify flags are false by default.
        assert!(!args.interactive);
        assert!(!args.tty);
        // Verify the name was parsed.
        assert_eq!(args.name, Some("my-dev-container".to_string()));
        // Verify the command vector.
        assert_eq!(args.command, vec!["bash", "-c", "echo hello"]);
        // Verify other optional fields are None.
        assert!(args.user.is_none());
        assert!(args.workdir.is_none());
    }

    /// Test that the command fails parsing if no command is provided after the options.
    #[test]
    fn test_exec_args_requires_command() {
        // Simulate `devrs env exec -it` (missing the command)
        let result = ExecArgs::try_parse_from(&["exec", "-it"]);
        // Expect an error because the command is required.
        assert!(result.is_err(), "Should fail without a command");
    }

    // Note: Integration tests for `handle_exec` would require mocking:
    // 1. `config::load_config`
    // 2. `common::docker::lifecycle::ensure_core_env_running`
    // 3. `common::docker::interaction::exec_in_container` (to simulate different exit codes)
    // Then, the test could verify that the correct arguments are passed to the Docker utilities
    // and that the final Result (Ok or Err) matches the simulated exit code.
}
