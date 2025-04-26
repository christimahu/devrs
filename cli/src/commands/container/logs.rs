//! # DevRS Container Logs Handler
//!
//! File: cli/src/commands/container/logs.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs container logs` subcommand, which fetches
//! and displays logs from a specified application container. It provides a
//! simplified interface to Docker's container logging functionality.
//!
//! ## Architecture
//!
//! The implementation follows these steps:
//!
//! 1. Parse command-line arguments for container name, follow flag, and line count.
//! 2. Validate the line count value, defaulting to "100" if invalid or accepting "all".
//! 3. Use the shared Docker interaction utility (`common::docker::interaction::get_container_logs`)
//!    to fetch and stream container logs to the console.
//! 4. Display logs to stdout with appropriate formatting provided by the Docker utility.
//!
//! ## Usage
//!
//! ```bash
//! # Display logs from a container (default: last 100 lines)
//! devrs container logs my-container
//!
//! # Follow logs in real-time (streaming)
//! devrs container logs -f my-container
//!
//! # Specify number of lines to show
//! devrs container logs -n 500 my-container
//!
//! # Show all logs
//! devrs container logs --lines all my-container
//! ```
//!
//! The command handles various edge cases such as the container not existing
//! (via the underlying Docker utility) and streaming interruptions.
//!
use crate::{
    common::docker, // Access shared Docker utilities.
    core::error::Result, // Standard Result type for error handling.
};
use anyhow::Context; // For adding context to errors.
use clap::Parser; // For parsing command-line arguments.
use tracing::{info, warn}; // Logging framework utilities.

/// # Container Logs Arguments (`LogsArgs`)
///
/// Defines the command-line arguments accepted by the `devrs container logs` subcommand.
/// Uses the `clap` crate for parsing and validation.
#[derive(Parser, Debug)]
#[command(about = "Fetch logs from a specified application container")]
pub struct LogsArgs {
    /// The name or ID of the target application container from which to fetch logs.
    /// This argument is required.
    #[arg(required = true)]
    container_name_or_id: String,

    /// Optional: If set, continuously stream new log output after displaying existing logs.
    /// Equivalent to `docker logs -f`.
    #[arg(short, long)] // Define as `--follow` or `-f`
    follow: bool,

    /// Optional: Number of lines to show from the end of the logs.
    /// Accepts a specific number (e.g., "500") or the string "all".
    /// Defaults to "100" if not specified or if an invalid numeric value is given.
    #[arg(long, short = 'n', default_value = "100")] // Define as `--lines` or `-n`
    lines: String,
}

/// # Handle Container Logs Command (`handle_logs`)
///
/// The main asynchronous handler function for the `devrs container logs` command.
/// It takes the parsed arguments, validates the log tailing options, and calls
/// the shared Docker utility function to stream logs from the specified container.
///
/// ## Workflow:
/// 1.  Logs the command execution details.
/// 2.  Validates the `--lines` argument:
///     * If "all" (case-insensitive), uses "all".
///     * If a valid positive integer, uses the provided number string.
///     * Otherwise, logs a warning and defaults to "100".
/// 3.  Prints an informational message indicating which container's logs are being fetched.
/// 4.  Calls `common::docker::interaction::get_container_logs`, passing the container name/ID,
///     the `follow` flag, and the validated `tail` option. This function handles the
///     actual Docker API interaction and streams output to stdout.
/// 5.  If not following logs (`--follow` was false), prints a completion message after the log stream ends.
///
/// ## Arguments
///
/// * `args`: The parsed `LogsArgs` struct containing the container name/ID and log options.
///
/// ## Returns
///
/// * `Result<()>`: Returns `Ok(())` if logs were successfully streamed (or if the stream ended cleanly).
/// * `Err`: Returns an `Err` if the underlying Docker utility fails (e.g., container not found, Docker API error).
pub async fn handle_logs(args: LogsArgs) -> Result<()> {
    // Log command details.
    info!(
        "Handling container logs command for '{}' (Follow: {}, Lines: {})",
        args.container_name_or_id, args.follow, args.lines
    );

    // Validate the `--lines` argument and determine the `tail` value for the Docker API.
    let tail: Option<&str> = if args.lines.trim().eq_ignore_ascii_case("all") {
        // User explicitly requested all lines.
        Some("all")
    } else if args.lines.trim().parse::<u32>().is_ok() {
        // User provided a valid number, pass it as a string slice.
        // `get_container_logs` expects `Option<&str>`.
        Some(&args.lines)
    } else {
        // Input was not "all" and not a valid number. Log a warning and use the default.
        warn!(
            "Invalid value for --lines: '{}'. Defaulting to '100'.",
            args.lines
        );
        Some("100") // Default to 100 lines.
    };

    // Inform the user what's happening.
    println!(
        "Fetching logs for container '{}'{}...",
        args.container_name_or_id,
        if args.follow { " (following)" } else { "" } // Indicate if following.
    );

    // Call the shared Docker utility function to get logs.
    // This function handles the connection, API call, and streaming output to stdout.
    docker::interaction::get_container_logs(
        &args.container_name_or_id, // Pass the container name/ID.
        args.follow,                // Pass the follow flag.
        tail,                       // Pass the validated tail option.
    )
    .await // Await the async operation.
    .with_context(|| { // Add context to any potential error from the utility function.
        format!(
            "Failed to get logs for container '{}'",
            args.container_name_or_id
        )
    })?;

    // If we were not following the logs, print a completion message after the stream finishes.
    // If following, the stream typically only ends when interrupted (Ctrl+C) or the container stops.
    if !args.follow {
        println!(
            "\nFinished displaying logs for container '{}'.",
            args.container_name_or_id
        );
    }

    Ok(()) // Indicate successful execution.
}


// --- Unit Tests ---
// Focus on testing the argument parsing logic for this specific command.
// Testing the `handle_logs` logic itself would require mocking the `docker::interaction::get_container_logs` call.
#[cfg(test)]
mod tests {
    use super::*;

    /// Test parsing with various flags and the required container name.
    #[test]
    fn test_logs_args_parsing() {
        // Simulate `devrs container logs my-app-container -f --lines 500`
        let args = LogsArgs::try_parse_from(["logs", "my-app-container", "-f", "--lines", "500"])
            .unwrap();
        // Verify the required name was parsed correctly.
        assert_eq!(args.container_name_or_id, "my-app-container");
        // Verify the --follow flag was parsed.
        assert!(args.follow);
        // Verify the --lines argument was parsed.
        assert_eq!(args.lines, "500");
    }

    /// Test that the command fails parsing if the required container name is missing.
    #[test]
    fn test_logs_args_requires_name() {
        // Simulate `devrs container logs` (without the container name)
        let result = LogsArgs::try_parse_from(["logs"]);
        // Expect an error because the container name is required.
        assert!(result.is_err(), "Should fail without container name");
    }

    // Note: Testing the `handle_logs` function's internal logic (like the `tail` validation
    // or the call to the Docker utility) would require mocking `docker::interaction::get_container_logs`.
    // These tests focus solely on the command-line argument parsing handled by `clap`.
}
