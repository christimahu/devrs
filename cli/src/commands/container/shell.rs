//! # DevRS Container Shell Handler
//!
//! File: cli/src/commands/container/shell.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs container shell` subcommand. Its primary
//! purpose is to provide an **interactive shell** within a **temporary container**
//! created from a specified Docker image. This is mainly intended for **debugging
//! and exploring the contents and behavior** of an application image without
//! affecting any persistently running containers.
//!
//! ## Architecture
//!
//! The command flow involves these key steps:
//! 1.  Parse command-line arguments (`ShellArgs`) for the target image name and optional command override.
//! 2.  Verify the specified image exists locally using `common::docker::images::image_exists`.
//! 3.  Determine the command to execute inside the container: defaults to `/bin/sh` if none is provided by the user.
//! 4.  Generate a temporary, semi-unique name for the container session.
//! 5.  Call the shared `common::docker::operations::run_container` utility with specific options:
//!     * Runs the container in the **foreground** (`--detach=false`).
//!     * Enables **interactive mode** with a TTY attached (like `docker run -it`).
//!     * Sets **auto-removal** (`--rm=true`) so the container is automatically cleaned up when the shell exits.
//!     * Overrides the image's default command with the one determined in step 3.
//!     * Does *not* map any ports or mount any volumes by default, providing isolation.
//! 6.  The `run_container` function handles the interactive session and returns the exit code.
//! 7.  A final message confirms the shell exit and container removal.
//!
//! ## Usage
//!
//! ```bash
//! # Start a default shell (/bin/sh) in a container from my-image:latest
//! devrs container shell my-image:latest
//!
//! # Start a specific shell (e.g., bash) in the container
//! devrs container shell my-image:latest bash
//!
//! # Run a different command entirely instead of a shell
//! devrs container shell my-image:latest python -V
//! devrs container shell my-image:latest ls -la /app
//! ```
//!
//! **Important:** This command creates a *new, temporary container* each time it's run.
//! The container is discarded upon exiting the shell. It does *not* connect to an existing
//! running container. For that, consider using `devrs container exec` or `docker exec`.
//!
use crate::{
    common::docker, // Access shared Docker utilities (image_exists, run_container).
    core::error::Result, // Standard Result type for error handling.
};
use anyhow::{anyhow, Context}; // For error creation and adding context.
use clap::Parser; // For parsing command-line arguments.
use tracing::{debug, info, warn}; // Logging framework utilities.

/// # Container Shell Arguments (`ShellArgs`)
///
/// Defines the command-line arguments accepted by the `devrs container shell` subcommand.
/// Uses the `clap` crate for parsing and validation.
#[derive(Parser, Debug)]
#[command(
    about = "Start an interactive shell in a container built from a specified image (for debugging)",
    long_about = "Starts a *temporary*, interactive container from the specified image and attaches a shell.\n\
                  The container is automatically removed when the shell exits.\n\
                  This is primarily intended for debugging image contents.\n\
                  WARNING: This provides direct access to the image filesystem." // Simplified warning
)]
pub struct ShellArgs {
    /// The name (and optionally tag) of the Docker image to create the temporary container from
    /// (e.g., "my-app:latest", "ubuntu:22.04"). This argument is required.
    #[arg(required = true)]
    image_name: String,

    // Force flag and confirmation prompt were removed for simplification.
    // #[arg(long, short)]
    // force: bool,

    /// Optional: Specifies the shell or command to run inside the container.
    /// If omitted, it defaults to "/bin/sh".
    /// If provided, the first word is treated as the command, and subsequent words
    /// are passed as arguments to that command.
    /// Example: `bash -l` would run `/bin/bash` with the `-l` argument.
    /// Example: `python` would run the python interpreter.
    #[arg(last = true)] // Captures all arguments after the image_name.
    command: Vec<String>,
}

/// # Handle Container Shell Command (`handle_shell`)
///
/// The main asynchronous handler function for the `devrs container shell` command.
/// It validates the input image, prepares arguments, and launches a temporary,
/// interactive container session using the specified image. The container is
/// automatically removed upon exit.
///
/// ## Workflow:
/// 1.  Logs the start and parsed arguments.
/// 2.  Checks if the specified `image_name` exists locally using `docker::images::image_exists`. If not, returns an `ImageNotFound` error.
/// 3.  Determines the command to execute inside the container: uses the user-provided `command` vector if not empty, otherwise defaults to `vec!["/bin/sh"]`.
/// 4.  Generates a temporary container name (e.g., `devrs-shell-tmp-<sanitized-image-name>`) to avoid conflicts, logging a warning about potential (though unlikely) collisions.
/// 5.  Calls `docker::operations::run_container` with specific arguments tailored for this command:
///     * `image_name`: The user-specified image.
///     * `temp_container_name`: The generated temporary name.
///     * `ports`: Empty (no port mapping needed for a debug shell).
///     * `mounts`: Empty (no volume mounting by default for isolation).
///     * `env_vars`: Empty (no extra environment variables).
///     * `workdir`: Defaults to "/" (obtained via `get_default_workdir`).
///     * `detach`: `false` (run in foreground).
///     * `rm`: `true` (automatically remove container on exit).
///     * `command`: The command determined in step 3.
/// 6.  The `run_container` call handles the interactive session and returns `Ok(())` only if the command inside the container exits with status code 0. If the command exits non-zero, `run_container` returns an `Err`.
/// 7.  If `run_container` succeeds, prints a confirmation message that the shell exited and the container was removed.
///
/// ## Arguments
///
/// * `args`: The parsed `ShellArgs` struct containing the image name and optional command override.
///
/// ## Returns
///
/// * `Result<()>`: Returns `Ok(())` if the shell session completes successfully (exit code 0).
/// * `Err`: Returns an `Err` if the image is not found, the Docker operation fails, or the shell/command inside the container exits with a non-zero status code.
pub async fn handle_shell(args: ShellArgs) -> Result<()> {
    // Log entry point and key arguments.
    info!(
        "Handling container shell command (Image: {}, Command: {:?})",
        args.image_name, args.command
    );

    // 1. Check if the target image exists locally. Error out if not found.
    if !docker::images::image_exists(&args.image_name).await? { //
        // Use the specific error type for clarity.
        return Err(anyhow!(crate::core::error::DevrsError::ImageNotFound {
            name: args.image_name
        }));
    }
    info!("Image '{}' found locally.", args.image_name); // Log success if found.

    // 2. Determine the command to execute inside the container.
    let cmd_to_run = if args.command.is_empty() {
        // If the user didn't provide a command, default to /bin/sh.
        warn!(
            "No command specified, defaulting to '/bin/sh'. Ensure this shell exists in the image."
        );
        vec!["/bin/sh".to_string()] // Use a Vec<String> as required by run_container.
    } else {
        // Use the command and arguments provided by the user.
        args.command.clone()
    };
    debug!("Command to execute in container: {:?}", cmd_to_run); // Log the chosen command.

    // 3. Generate a temporary name for the container.
    // This aims to avoid conflicts but isn't guaranteed unique across rapid calls.
    // Sanitize the image name to remove characters invalid in container names.
    let temp_container_name = format!(
        "devrs-shell-tmp-{}",
        args.image_name.replace([':', '/'], "-") // Replace common invalid chars.
    );
    // Warn the user about the temporary nature and potential for name conflicts (rare).
    warn!(
        "Using temporary container name: {}. If conflicts occur, consider manual removal.",
        temp_container_name
    );
    debug!("Using temporary container name: {}", temp_container_name); // Log the name.

    // 4. Execute the interactive shell/command within a new, temporary container.
    println!(
        "Starting interactive shell in image '{}' (container: {})...",
        args.image_name, temp_container_name
    );

    // Call the shared run_container utility with specific flags for this command:
    // - image_name: User-specified image.
    // - temp_container_name: Generated name.
    // - ports/mounts/env_vars: Empty defaults for isolation.
    // - workdir: Default determined by helper function.
    // - detach: false (foreground interactive session).
    // - rm: true (auto-remove container on exit).
    // - command: The shell/command determined in step 2.
    docker::operations::run_container( //
        &args.image_name,
        &temp_container_name,
        &[],                                                // No ports.
        &[],                                                // No mounts.
        &Default::default(),                                // No extra env vars.
        Some(&get_default_workdir(&args.image_name).await), // Default workdir (currently "/").
        false,                                              // Run in foreground (not detached).
        true,                                               // Auto-remove container on exit.
        Some(cmd_to_run.clone()),                           // Command to run inside.
    )
    .await // Await the container execution.
    .with_context(|| { // Add context if the run operation itself fails.
        format!(
            "Failed to start interactive shell in image '{}'",
            args.image_name
        )
    })?;
    // Note: `run_container` should ideally return an Err if the executed command
    // inside the container returns a non-zero exit code. If it returns Ok here,
    // it implies the command exited successfully (status 0).

    // 5. Confirm exit and removal to the user.
    println!(
        "\nShell exited from image '{}' (container '{}' removed).",
        args.image_name, temp_container_name
    );

    Ok(()) // Indicate overall success of the command.
}

/// # Get Default Working Directory (`get_default_workdir`)
///
/// Determines a default working directory to use when starting the container shell.
/// Currently, it simply returns "/", but future enhancements could involve inspecting
/// the image's configured `WorkingDir` using `docker::images::inspect_image`.
///
/// ## Arguments
///
/// * `_image_name`: The name of the image (currently unused, but kept for future use).
///
/// ## Returns
///
/// * `String`: The default working directory path (currently always "/").
async fn get_default_workdir(_image_name: &str) -> String {
    // TODO: Future enhancement: Inspect the image configuration:
    // let image_inspect = docker::images::inspect_image(image_name).await?;
    // if let Some(config) = image_inspect.config {
    //     if let Some(wd) = config.working_dir {
    //         if !wd.is_empty() {
    //             return wd;
    //         }
    //     }
    // }
    // Fallback to root directory if inspection fails or WorkingDir is not set.
    "/".to_string()
}


// --- Unit Tests ---
// Focus on argument parsing. Testing the handler requires mocking Docker interactions.
#[cfg(test)]
mod tests {
    use super::*;

    /// Test parsing arguments including an override command and its arguments.
    #[test]
    fn test_shell_args_parsing_with_command() {
        // Simulate `devrs container shell myimage:debug python -V`
        // Note: Force flag was removed, so it's not included here.
        let args = ShellArgs::try_parse_from([
            "shell",         // Command name context for clap.
            "myimage:debug", // Required image name.
            "--", // Add separator before trailing command args
            "python",        // Start of the override command.
            "-V",            // Argument for the override command.
        ])
        .unwrap(); // Expect parsing to succeed.

        // Verify the parsed arguments.
        assert_eq!(args.image_name, "myimage:debug");
        // assert!(args.force); // Force flag was removed.
        assert_eq!(args.command, vec!["python", "-V"]); // Check override command vector.
    }

    /// Test parsing arguments without an override command (should use default shell).
    #[test]
    fn test_shell_args_parsing_default_shell() {
         // Simulate `devrs container shell myimage:debug`
        let args = ShellArgs::try_parse_from([
            "shell",         // Command name context.
            "myimage:debug", // Required image name.
                             // No override command provided.
        ])
        .unwrap(); // Expect parsing to succeed.

        // Verify parsed arguments.
        assert_eq!(args.image_name, "myimage:debug");
        // assert!(!args.force); // Force flag was removed.
        // The command vector should be empty when no override is given.
        assert!(args.command.is_empty());
    }

    /// Test that the command fails parsing if the required image name is missing.
    #[test]
    fn test_shell_args_requires_name() {
        // Simulate `devrs container shell` (missing image name)
        let result = ShellArgs::try_parse_from(["shell"]);
        // Expect an error because the image name is required.
        assert!(result.is_err(), "Should fail without image name");
    }

    // Note: Integration tests for `handle_shell` would need to mock:
    // 1. `docker::images::image_exists` -> To simulate image presence/absence.
    // 2. `docker::operations::run_container` -> To simulate container startup, interactive session, exit code, and removal, verifying the correct arguments (`rm=true`, `detach=false`, etc.) were passed.
}
