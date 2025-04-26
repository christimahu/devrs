//! # DevRS Environment Logs Handler
//!
//! File: cli/src/commands/env/logs.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs env logs` subcommand. Its purpose is to
//! fetch and display logs specifically from the **core development environment**
//! container. This allows users to view the output of processes running within
//! that shared environment container.
//!
//! ## Architecture
//!
//! The command flow involves these steps:
//! 1. Parse command-line arguments (`LogsArgs`) using `clap`, including flags for following (`-f`), number of lines (`-n`), and optionally overriding the container name (`--name`).
//! 2. Load the DevRS configuration (`core::config`) to determine the default name of the core environment container if not specified via `--name`.
//! 3. Determine the final target container name.
//! 4. Validate the value provided for `--lines` (accepting "all" or numbers, defaulting to "100").
//! 5. Call the shared Docker utility function `common::docker::interaction::get_container_logs`, passing the container name, follow flag, and tail option. This function handles the Docker API call and streams the log output to the console.
//! 6. If not following logs (`-f` was not used), print a concluding message after the logs are displayed.
//!
//! ## Usage
//!
//! ```bash
//! # Show recent logs (default 100 lines) from the default core env container
//! devrs env logs
//!
//! # Follow logs in real-time
//! devrs env logs -f
//!
//! # Specify number of lines to show
//! devrs env logs -n 500
//!
//! # Show all logs
//! devrs env logs -n all
//! # Equivalent:
//! devrs env logs --lines all
//!
//! # Show logs from a specifically named core env container (if using non-default name)
//! devrs env logs --name my-custom-core-env-instance
//! ```
//!
//! This command interacts specifically with the core environment container, unlike
//! `devrs container logs` which targets application-specific containers.
//!
use crate::{
    common::docker::{self}, // Access shared Docker utilities (interaction::get_container_logs).
    core::{
        config,        // Access configuration loading.
        error::Result, // Standard Result type for error handling.
    },
};
use anyhow::Context; // For adding context to errors.
use clap::Parser; // For parsing command-line arguments.
use tracing::{debug, info, warn}; // Logging framework utilities.

/// # Environment Logs Arguments (`LogsArgs`)
///
/// Defines the command-line arguments accepted by the `devrs env logs` subcommand.
/// Uses the `clap` crate for parsing and validation.
#[derive(Parser, Debug)]
#[command(about = "View logs from the core development environment container")]
pub struct LogsArgs {
    /// Optional: If set, continuously stream new log output after displaying existing logs.
    /// Equivalent to `docker logs -f`.
    #[arg(short, long)] // Define as `-f` or `--follow`.
    follow: bool,

    /// Optional: Specifies the number of lines to show from the end of the logs.
    /// Accepts a specific number (e.g., "500") or the literal string "all".
    /// Defaults to "100" lines if omitted or if an invalid numeric value is provided.
    #[arg(long, short = 'n', default_value = "100")]
    // Define as `--lines` or `-n`, with default.
    lines: String,

    /// Optional: Specifies the exact name of the core environment container to fetch logs from.
    /// If omitted, the default name (derived from the `core_env.image_name` in the configuration,
    /// typically `<image_name>-instance`) is used.
    #[arg(long)] // Define as `--name <NAME>`.
    name: Option<String>,
}

/// # Handle Environment Logs Command (`handle_logs`)
///
/// The main asynchronous handler function for the `devrs env logs` command.
/// It identifies the target core environment container, validates log options,
/// and then uses the shared Docker utility to stream the logs to the console.
///
/// ## Workflow:
/// 1.  Logs the command start and parsed arguments.
/// 2.  Loads the DevRS configuration to get core environment details (needed for default container name).
/// 3.  Determines the target container name: uses `--name` if provided, otherwise generates the default name using `get_core_env_container_name`.
/// 4.  Validates the `--lines` argument value (`args.lines`), determining the appropriate `tail` value (`Some("all")`, `Some("number_string")`, or `Some("100")`) to pass to the Docker utility. Logs a warning if defaulting.
/// 5.  Prints an informational message to the user about which container's logs are being fetched.
/// 6.  Calls `common::docker::interaction::get_container_logs` with the determined container name, follow flag, and validated tail option. This handles the Docker API call and output streaming.
/// 7.  If `follow` was false, prints a confirmation message after the log stream ends.
///
/// ## Arguments
///
/// * `args`: The parsed `LogsArgs` struct containing the command-line options.
///
/// ## Returns
///
/// * `Result<()>`: Returns `Ok(())` if logs were successfully streamed or if the stream ended cleanly.
/// * `Err`: Returns an `Err` if config loading fails, the container isn't found (error propagated from the Docker utility), or the Docker API interaction fails.
pub async fn handle_logs(args: LogsArgs) -> Result<()> {
    info!("Handling env logs command..."); // Log entry point.
    debug!("Logs args: {:?}", args); // Log parsed arguments.

    // 1. Load configuration - needed to determine the default container name if --name is not used.
    let cfg = config::load_config().context("Failed to load DevRS configuration")?;

    // 2. Determine the target core environment container name.
    let container_name = args.name.clone().unwrap_or_else(|| {
        // If --name wasn't provided, generate the default name based on config.
        let default_name = get_core_env_container_name(&cfg);
        debug!("No specific name provided, using default: {}", default_name);
        default_name
    });

    // 3. Check if the container exists before attempting to get logs.
    // While `get_container_logs` also checks, checking here provides a slightly earlier and potentially clearer warning.
    if !docker::state::container_exists(&container_name).await? {
        //
        // Log a warning. The subsequent call to `get_container_logs` will return the proper error.
        warn!("Container '{}' not found.", container_name);
        // Note: We proceed here and let `get_container_logs` return the ContainerNotFound error
        // for consistent error handling from the Docker interaction layer.
    }

    // 4. Validate the `--lines` argument and determine the `tail` value for the Docker API.
    let tail: Option<&str> = if args.lines.trim().eq_ignore_ascii_case("all") {
        // User explicitly asked for all lines.
        Some("all")
    } else if args.lines.trim().parse::<u32>().is_ok() {
        // User provided a valid number string. Pass it as a string slice.
        Some(args.lines.as_str()) // Borrow the string slice from args.lines.
    } else {
        // Input was neither "all" nor a valid number. Warn and use the default.
        warn!(
            "Invalid value for --lines: '{}'. Using default '100'.",
            args.lines
        );
        Some("100") // Default to 100 lines.
    };

    // 5. Inform the user about the operation.
    println!(
        "Fetching logs for core env container '{}'{}...",
        container_name,
        if args.follow { " (following)" } else { "" } // Indicate if following.
    );

    // 6. Call the shared Docker utility to get and stream logs.
    // This handles the actual Docker API communication and printing to stdout.
    docker::interaction::get_container_logs(
        &container_name, // Pass the determined container name.
        args.follow,     // Pass the follow flag.
        tail,            // Pass the validated tail option.
    )
    .await // Await the async log streaming operation.
    .with_context(|| format!("Failed to get logs for container '{}'", container_name))?; // Add context on error.

    // 7. Print completion message if not following.
    // If following, the stream usually only terminates via Ctrl+C or container stop.
    if !args.follow {
        println!(
            "\nFinished displaying logs for container '{}'.",
            container_name
        );
    }

    Ok(()) // Indicate overall success.
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
// Focus on argument parsing for the `env logs` command. Testing the handler
// logic requires mocking config loading and Docker API interactions.
#[cfg(test)]
mod tests {
    use super::*;

    /// Test parsing with default arguments (no flags or name).
    #[test]
    fn test_logs_args_parsing_defaults() {
        // Simulate `devrs env logs`
        let args = LogsArgs::try_parse_from(["logs"]).unwrap(); // Use "logs" as command name context.
                                                                // Verify default values.
        assert!(!args.follow); // Default is false.
        assert_eq!(args.lines, "100"); // Default is "100".
        assert!(args.name.is_none()); // Default is None.
    }

    /// Test parsing with all optional flags and arguments provided.
    #[test]
    fn test_logs_args_parsing_with_options() {
        // Simulate `devrs env logs --follow -n 50 --name my-dev-container`
        let args = LogsArgs::try_parse_from([
            "logs",
            "--follow",         // Follow flag.
            "-n",               // Short lines flag.
            "50",               // Lines value.
            "--name",           // Name flag.
            "my-dev-container", // Name value.
        ])
        .unwrap(); // Expect parsing to succeed.

        // Verify parsed values.
        assert!(args.follow);
        assert_eq!(args.lines, "50");
        assert_eq!(args.name, Some("my-dev-container".to_string()));
    }

    /// Test parsing with "--lines all".
    #[test]
    fn test_logs_args_parsing_lines_all() {
        // Simulate `devrs env logs --lines all`
        let args = LogsArgs::try_parse_from(["logs", "--lines", "all"]).unwrap();
        assert_eq!(args.lines, "all");
        // Check other defaults.
        assert!(!args.follow);
        assert!(args.name.is_none());
    }

    // Note: Testing the `handle_logs` function's internal logic (like the `tail` validation,
    // container name generation, or the call to the Docker utility) would require mocking
    // `config::load_config` and `docker::interaction::get_container_logs`.
    // These tests focus solely on the command-line argument parsing handled by `clap`.
}
