//! # DevRS Docker Container Interaction
//!
//! File: cli/src/common/docker/interaction.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!

//! ## Overview
//!
//! This module provides functions for interacting with running or stopped Docker containers.
//! It allows executing commands within a container (`exec_in_container`) and streaming
//! logs from a container (`get_container_logs`). These are essential for commands like
//! `devrs env exec`, `devrs env shell`, `devrs container logs`, etc.
//!
//! ## Architecture
//!
//! Key functions implemented:
//! - **`exec_in_container`**: Executes a specified command inside a container.
//!   - Automatically starts the container if it's stopped.
//!   - Handles attaching `stdin`, `stdout`, and `stderr` based on `interactive` and `tty` flags, enabling interactive sessions.
//!   - Allows specifying the user and working directory for the command execution context.
//!   - Waits for the command to complete and returns its exit code.
//! - **`get_container_logs`**: Streams logs (stdout/stderr) from a specified container.
//!   - Supports following logs in real-time (`follow` flag).
//!   - Allows specifying the number of trailing lines to fetch (`tail` option).
//!   - Prints logs directly to the host's standard output.
//!
//! Both functions utilize asynchronous I/O (`tokio`) and the `bollard` crate to manage
//! communication streams with the Docker daemon and the container processes. Error handling
//! maps Docker API errors to specific `DevrsError` types.
//!
//! ## Usage
//!
//! ```rust
//! use crate::common::docker::interaction;
//! use crate::core::error::Result;
//!
//! # async fn run_examples() -> Result<()> {
//! let container_name = "my-app"; // Assume this container exists
//!
//! // Example 1: Execute a non-interactive command
//! let ls_command = vec!["ls".to_string(), "-la".to_string(), "/app".to_string()];
//! let exit_code = interaction::exec_in_container(
//!     container_name,
//!     &ls_command,
//!     false, // interactive
//!     false, // tty
//!     None,  // workdir
//!     None   // user
//! ).await?;
//! println!("'ls -la /app' exited with code: {}", exit_code);
//!
//! // Example 2: Start an interactive bash shell
//! let shell_command = vec!["/bin/bash".to_string()];
//! interaction::exec_in_container(
//!     container_name,
//!     &shell_command,
//!     true, // interactive
//!     true, // tty
//!     Some("/app"), // workdir
//!     None // user
//! ).await?; // Returns when shell exits
//!
//! // Example 3: Get the last 20 lines of logs
//! interaction::get_container_logs(container_name, false, Some("20")).await?;
//!
//! // Example 4: Follow logs continuously
//! // interaction::get_container_logs(container_name, true, None).await?; // This would block
//! # Ok(())
//! # }
//! ```
//!
use crate::core::error::{DevrsError, Result}; // Use Result/Error from core module
use anyhow::{anyhow, Context}; // For error context
use bollard::{
    container::{LogOutput, LogsOptions}, // Use specific container types for logs
    exec::{CreateExecOptions, StartExecResults}, // Types for exec operations
};
use futures_util::StreamExt; // Required for processing streams (like logs or exec output)
use std::{
    default::Default,  // For default struct initializers
    io::{self, Write}, // Standard IO traits (used for stdout flushing)
    time::Duration,    // For specifying delays (e.g., after starting container)
};
use tokio::{
    io::{copy, stderr, stdin, stdout, AsyncWriteExt}, // Async IO operations
    task, // For spawning concurrent tasks (handling stdin/stdout/stderr for exec)
};
use tracing::{debug, error, info, instrument, warn}; // Logging framework utilities

// Import functions from sibling modules needed for exec/logs prerequisites.
use super::connect::connect_docker; // Get Docker client connection
use super::lifecycle::start_container; // Start container if stopped
use super::state::{container_exists, container_running}; // Check container status

/// Executes a command inside a specified container, handling interactivity.
///
/// This function ensures the target container is running (starting it if necessary)
/// and then creates and starts a Docker `exec` instance to run the provided command.
/// It manages the attachment of standard input, output, and error streams based on
/// the `interactive` and `tty` flags, similar to `docker exec -it`.
///
/// # Arguments
///
/// * `name_or_id` - The name or ID of the target container.
/// * `cmd` - A slice of strings representing the command and its arguments (e.g., `&["ls", "-la"]`).
/// * `interactive` - If `true`, the host's standard input is attached to the command's standard input.
/// * `tty` - If `true`, a pseudo-terminal (TTY) is allocated for the exec instance. This is typically required for interactive shell sessions.
/// * `workdir` - An optional string slice specifying the working directory inside the container where the command should be executed. If `None`, the container's default working directory is used.
/// * `user` - An optional string slice specifying the username or UID to run the command as inside the container. If `None`, the container's default user is used.
///
/// # Returns
///
/// * `Result<i64>` - On successful execution, returns `Ok(exit_code)` where `exit_code` is the integer exit code of the executed command within the container. Returns `-1` if the exit code couldn't be determined from the Docker API response.
///
/// # Errors
///
/// * `DevrsError::ContainerNotFound` - If the specified container does not exist.
/// * `DevrsError::DockerOperation` - If the container exists but is stopped and fails to start.
/// * `DevrsError::DockerApi` - For errors during communication with the Docker daemon (e.g., creating or starting the exec instance, inspecting the result).
#[instrument(skip(name_or_id, cmd, interactive, tty, workdir, user), fields(container = %name_or_id))] // Tracing span
#[allow(clippy::too_many_arguments)] // Necessary due to the number of options for exec
pub async fn exec_in_container(
    name_or_id: &str,
    cmd: &[String],
    interactive: bool,
    tty: bool,
    workdir: Option<&str>,
    user: Option<&str>,
) -> Result<i64> {
    // Establish connection to Docker daemon.
    let docker = connect_docker().await?;

    // --- Ensure Container is Running ---
    // Check current state.
    if !container_running(name_or_id).await? {
        // If not running, check if it exists but is stopped.
        if container_exists(name_or_id).await? {
            info!(
                "Container '{}' is stopped, attempting to start for exec...",
                name_or_id
            );
            // Attempt to start the stopped container.
            start_container(name_or_id).await.with_context(|| {
                format!(
                    "Failed to start stopped container '{}' before exec",
                    name_or_id
                )
            })?;
            // Brief pause to allow container entrypoint/processes to initialize.
            tokio::time::sleep(Duration::from_millis(500)).await;
            // Verify it actually started.
            if !container_running(name_or_id).await? {
                error!(
                    "Container '{}' failed to reach running state after start attempt.",
                    name_or_id
                );
                return Err(anyhow!(DevrsError::DockerOperation(format!(
                    "Container '{}' could not be started for exec.",
                    name_or_id
                ))));
            }
            info!("Container '{}' started successfully.", name_or_id);
        } else {
            // Container doesn't exist at all.
            warn!(
                "Exec failed because container '{}' was not found.",
                name_or_id
            );
            return Err(anyhow!(DevrsError::ContainerNotFound {
                name: name_or_id.to_string()
            }));
        }
    }

    // Log the exec creation details.
    info!(
        "Creating exec instance in container '{}' for command: {:?} (Interactive: {}, TTY: {})",
        name_or_id, cmd, interactive, tty
    );

    // --- Create Exec Instance ---
    // Define options for the Docker `exec_create` API call.
    let exec_options = CreateExecOptions {
        attach_stdout: Some(true),              // Always attach stdout.
        attach_stderr: Some(true),              // Always attach stderr.
        attach_stdin: Some(interactive),        // Attach stdin only if interactive flag is true.
        tty: Some(tty),                         // Allocate TTY if tty flag is true.
        cmd: Some(cmd.to_vec()),                // The command and arguments to run.
        working_dir: workdir.map(String::from), // Optional working directory.
        user: user.map(String::from),           // Optional user.
        ..Default::default()                    // Use defaults for other options (e.g., Env).
    };

    // Make the API call to create the exec instance.
    let exec_create_response = docker
        .create_exec(name_or_id, exec_options)
        .await
        // Map potential errors.
        .map_err(|e| match e {
            // Handle container not found specifically.
            bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            } => anyhow!(DevrsError::ContainerNotFound {
                name: name_or_id.to_string()
            }),
            // Handle other API errors generally.
            _ => anyhow!(DevrsError::DockerApi { source: e }).context(format!(
                "Failed to create exec instance in container '{}'",
                name_or_id
            )),
        })?;

    // Get the ID assigned to the new exec instance.
    let exec_id = exec_create_response.id;
    info!("Created exec instance ID: {}", exec_id);

    // --- Start Exec Instance and Handle I/O ---
    // Make the API call to start the previously created exec instance.
    let start_exec_result = docker
        .start_exec(&exec_id, None) // No specific start options needed here.
        .await
        .map_err(|e| {
            anyhow!(DevrsError::DockerApi { source: e }).context("Failed to start exec instance")
        })?;

    // Process the result of starting the exec instance.
    // It returns different types depending on whether streams were attached.
    match start_exec_result {
        // Case 1: Streams are attached (stdout, stderr, potentially stdin).
        StartExecResults::Attached {
            mut output, // Multiplexed stream for stdout/stderr from the container.
            mut input,  // Write stream for sending stdin to the container.
        } => {
            info!("Exec instance '{}' attached. Streaming stdio...", exec_id);

            // --- Stdin Handling Task ---
            // Spawn a concurrent task to copy data from host stdin to container input stream if interactive.
            let stdin_handle = if interactive {
                task::spawn(async move {
                    let mut host_stdin = stdin(); // Get handle to host stdin.
                                                  // Copy bytes asynchronously.
                    match copy(&mut host_stdin, &mut input).await {
                        Ok(n) => debug!("Exec stdin stream finished after {} bytes.", n),
                        // Ignore BrokenPipe errors, common when remote end closes.
                        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                            debug!("Exec stdin broken pipe.")
                        }
                        Err(e) => warn!("Error writing stdin to exec: {}", e),
                    }
                    // Attempt to gracefully shut down the write half of the stream.
                    if let Err(e) = input.shutdown().await {
                        debug!("Error shutting down exec stdin writer: {}", e);
                    }
                })
            } else {
                // If not interactive, spawn a dummy task handle to avoid issues with `tokio::join!`.
                task::spawn(async {})
            };

            // --- Stdout/Stderr Handling Task ---
            // Spawn a concurrent task to read from the container's output stream
            // and write to the corresponding host stdout/stderr.
            let output_handle = task::spawn(async move {
                let mut host_stdout = stdout(); // Handle to host stdout.
                let mut host_stderr = stderr(); // Handle to host stderr.

                // Loop while the output stream has data.
                while let Some(result) = output.next().await {
                    match result {
                        // Successfully received a chunk of output.
                        Ok(log_output) => match log_output {
                            // Demultiplex the stream based on type.
                            LogOutput::StdOut { message } => {
                                // Write stdout chunk to host stdout.
                                if let Err(e) = host_stdout.write_all(&message).await {
                                    warn!("Error writing exec stdout to host stdout: {}", e);
                                    break; // Stop processing on write error.
                                }
                                // Flush to ensure visibility.
                                if let Err(e) = host_stdout.flush().await {
                                    warn!("Error flushing host stdout: {}", e);
                                }
                            }
                            LogOutput::StdErr { message } => {
                                // Write stderr chunk to host stderr.
                                if let Err(e) = host_stderr.write_all(&message).await {
                                    warn!("Error writing exec stderr to host stderr: {}", e);
                                    break; // Stop processing on write error.
                                }
                                // Flush to ensure visibility.
                                if let Err(e) = host_stderr.flush().await {
                                    warn!("Error flushing host stderr: {}", e);
                                }
                            }
                            // Handle other potential stream types (usually not expected for exec).
                            LogOutput::Console { message } => {
                                warn!("Exec Console Output: {}", String::from_utf8_lossy(&message));
                            }
                            LogOutput::StdIn { .. } => { /* Ignore stdin echoes */ }
                        },
                        // Error occurred while reading from the stream.
                        Err(e) => {
                            warn!("Error receiving output from exec stream: {}", e);
                            break; // Stop processing on stream error.
                        }
                    }
                }
                debug!("Exec output stream finished.");
                // Final flush of host streams.
                let _ = host_stdout.flush().await;
                let _ = host_stderr.flush().await;
            });

            debug!("Waiting for stdio tasks for exec '{}'...", exec_id);

            // --- Wait for I/O Tasks and Get Exit Code ---
            // Wait for both the stdin task (if active) and the output task to complete.
            let (stdin_res, output_res) = tokio::join!(stdin_handle, output_handle);
            // Log any errors (panics) from the I/O tasks.
            if let Err(e) = stdin_res {
                warn!("Stdin handling task failed for exec '{}': {}", exec_id, e);
            }
            if let Err(e) = output_res {
                warn!("Output handling task failed for exec '{}': {}", exec_id, e);
            }

            debug!("Stdio tasks finished for exec '{}'.", exec_id);

            // After I/O streams are closed/finished, inspect the exec instance again
            // to retrieve the final exit code of the command.
            debug!("Inspecting exec instance '{}' for exit code...", exec_id);
            match docker.inspect_exec(&exec_id).await {
                Ok(inspect_response) => {
                    // Extract the exit code. Default to -1 if it's unexpectedly None.
                    let exit_code = inspect_response.exit_code.unwrap_or(-1);
                    info!(
                        "Exec instance '{}' finished with exit code: {}",
                        exec_id, exit_code
                    );
                    Ok(exit_code) // Return the exit code.
                }
                Err(e) => {
                    // Error inspecting the exec instance after completion.
                    error!(
                        "Failed to inspect exec instance '{}' to get exit code: {}",
                        exec_id, e
                    );
                    Err(
                        anyhow!(DevrsError::DockerApi { source: e }).context(format!(
                            "Failed to inspect exec instance '{}' after execution",
                            exec_id
                        )),
                    )
                }
            }
        }
        // Case 2: Exec started in detached mode (no streams attached).
        StartExecResults::Detached => {
            info!("Exec instance '{}' started in detached mode.", exec_id);
            // In detached mode, we don't wait for completion or get an exit code here.
            // Return 0 to indicate the detached start itself was successful.
            // Callers needing the result of a detached command would need other mechanisms.
            Ok(0)
        }
    }
}

/// Retrieves logs from a specified container and streams them to the host's standard output.
///
/// This function connects to the Docker daemon, requests the logs for the given container,
/// and pipes the resulting stream directly to `stdout`. It supports options for following
/// new logs and specifying the number of recent lines to display initially.
///
/// # Arguments
///
/// * `name_or_id` - The name or ID of the target container.
/// * `follow` - If `true`, the function will continuously stream new log entries as they are generated by the container until the stream is interrupted (e.g., by Ctrl+C or the container stopping). If `false`, it fetches existing logs up to the specified `tail` limit and then exits.
/// * `tail` - An optional string slice specifying the number of lines to show from the end of the logs. Common values include:
///     - `"100"` (or any number): Show the last N lines.
///     - `"all"`: Show all logs from the beginning.
///     - `None`: Defaults internally to "100" lines.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if log streaming starts and completes without error (or is interrupted cleanly while following).
///
/// # Errors
///
/// * `DevrsError::ContainerNotFound` - If the specified container does not exist.
/// * `DevrsError::DockerApi` - For errors during communication with the Docker daemon or while processing the log stream.
#[instrument(skip(name_or_id, follow, tail), fields(container = %name_or_id))] // Tracing span
pub async fn get_container_logs(name_or_id: &str, follow: bool, tail: Option<&str>) -> Result<()> {
    // Establish connection to Docker daemon.
    let docker = connect_docker().await?;

    // Perform an upfront check for container existence for a clearer error message.
    if !container_exists(name_or_id).await? {
        warn!(
            "Cannot get logs because container '{}' was not found.",
            name_or_id
        );
        // Return the specific error immediately.
        return Err(anyhow!(DevrsError::ContainerNotFound {
            name: name_or_id.to_string()
        }));
    }

    // Log the requested action.
    info!(
        "Fetching logs for container '{}' (Follow: {}, Tail: {:?})",
        name_or_id,
        follow,
        tail.unwrap_or("default=100") // Log default clearly if None
    );

    // Prepare the `tail` option as an owned String required by `LogsOptions`.
    // Default to "100" if `tail` is `None`.
    let tail_owned = tail.unwrap_or("100").to_string();

    // Configure options for the Docker `logs` API call.
    let options = LogsOptions {
        stdout: true,         // Include stdout stream.
        stderr: true,         // Include stderr stream.
        follow,               // Follow new logs?
        tail: tail_owned,     // Number of lines from the end (or "all").
        timestamps: true,     // Include timestamps in the log output.
        ..Default::default()  // Use defaults for other options (e.g., since, until).
    };

    // Get the log stream from the Docker API.
    let mut log_stream = docker.logs(name_or_id, Some(options));
    // Get a handle to the host's standard output.
    let mut stdout_handle = io::stdout();

    // Process the log stream asynchronously.
    while let Some(log_result) = log_stream.next().await {
        match log_result {
            // Successfully received a log chunk (stdout or stderr).
            Ok(log_output) => {
                // Get the raw bytes from the LogOutput chunk.
                let bytes_to_write = log_output.into_bytes();
                // Write the bytes directly to the host's stdout.
                stdout_handle
                    .write_all(&bytes_to_write)
                    .context("Failed to write log chunk to stdout")?;
                // Flush stdout to ensure the output is immediately visible.
                stdout_handle.flush().context("Failed to flush stdout")?;
            }
            // Error occurred while reading from the log stream.
            Err(e) => {
                error!(
                    "Error receiving log stream for container '{}': {:?}",
                    name_or_id, e
                );
                // Propagate the error, wrapping it with context.
                // Note: Errors might occur if the container stops unexpectedly during follow.
                return Err(anyhow!(DevrsError::DockerApi { source: e })
                    .context(format!("Error reading logs for container '{}'", name_or_id)));
            }
        }
    }

    // Log completion message (primarily relevant when not following).
    if !follow {
        debug!("Finished streaming logs for container '{}'.", name_or_id);
    } else {
        // If following, this point is reached when the stream naturally ends (e.g., container stopped).
        debug!("Log stream following ended for container '{}'.", name_or_id);
    }

    Ok(()) // Indicate overall success.
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    // Note: Testing `exec_in_container` and `get_container_logs` effectively
    // requires significant mocking of the Docker connection, API responses,
    // and asynchronous I/O streams. These tests are typically complex.

    /// Placeholder test to ensure the module compiles.
    #[test]
    fn placeholder_interaction_test() {
        assert!(true);
    }

    // TODO: Add mocked tests for exec_in_container scenarios:
    // - Container stopped -> verify start_container is called.
    // - Container not found -> verify ContainerNotFound error.
    // - Interactive session -> verify stdin/stdout/stderr handling setup.
    // - Command exit code 0 -> verify Ok(0) return.
    // - Command exit code non-zero -> verify Err(DevrsError::ExternalCommand) return.

    // TODO: Add mocked tests for get_container_logs scenarios:
    // - Container not found -> verify ContainerNotFound error.
    // - Basic log fetch (no follow, default tail) -> verify LogsOptions and stream processing.
    // - Follow logs -> verify LogsOptions.
    // - Tail "all" / specific number -> verify LogsOptions.
}
