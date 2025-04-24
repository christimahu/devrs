//! # DevRS Static File Server
//!
//! File: cli/src/commands/srv/mod.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module provides a lightweight HTTP static file server for local development.
//! It serves files from a specified directory with configurable options for:
//! - CORS (Cross-Origin Resource Sharing)
//! - Port binding (with automatic fallback if port is in use)
//! - Host interface binding
//! - SPA (Single Page Application) mode
//! - Index file customization
//!
//! ## Architecture
//!
//! The module is organized into three key components:
//! - `config.rs`: Configuration loading and validation
//! - `server_logic.rs`: Core HTTP server implementation
//! - `utils.rs`: Utility functions for logging and system info
//!
//! The main `handle_srv` function serves as the entry point for the command,
//! processing arguments and launching the server.
//!
//! ## Examples
//!
//! Usage examples:
//!
//! ```bash
//! # Serve the current directory
//! devrs srv .
//!
//! # Specify a port and host interface
//! devrs srv --port 9000 --host 0.0.0.0 ./dist
//!
//! # Disable CORS and customize index file
//! devrs srv --no-cors --index home.html ./public
//! ```
//!
//! Server startup flow:
//! 1. Load and merge configuration from CLI args and config file
//! 2. Find available port (if requested port is in use)
//! 3. Set up router with static file serving and fallback handlers
//! 4. Display server URLs and start serving files
//!
use crate::core::error::Result; // Use the standard Result type for error handling.
use tracing::info; // Use the info macro for logging informational messages.

// --- Subcommand Argument Re-export ---
// Make the argument struct from the config module publicly available.
pub use config::SrvArgs;

// --- Submodule Declarations ---
// Declare the modules containing the implementation details for the server command.

/// Handles configuration loading and merging for the static file server.
pub mod config;

/// Contains the core Axum-based HTTP server implementation.
pub mod server_logic;

/// Provides utility functions, such as port checking and logging setup.
pub mod utils;

/// # Handle Server Command (`handle_srv`)
///
/// The main entry point function for the `devrs srv` command.
/// This asynchronous function orchestrates the server setup and execution.
///
/// It performs the following steps:
/// 1. Logs the reception of the command and its arguments.
/// 2. Loads and merges the server configuration using the `config` submodule. This combines
///    settings from command-line arguments and potentially a configuration file.
/// 3. Logs the final, effective configuration that will be used.
/// 4. Delegates the actual server execution (binding to a port, setting up routes, serving files)
///    to the `run_server` function within the `server_logic` submodule.
///
/// ## Arguments
///
/// * `args`: The parsed `SrvArgs` struct containing the command-line arguments provided by the user (e.g., port, directory, CORS settings).
///
/// ## Returns
///
/// * `Result<()>`: Propagates the `Result` from the configuration loading or server execution. Returns `Ok(())`
///   if the server starts and runs without critical errors during initialization, or an `Err` if configuration fails
///   or the server logic encounters an unrecoverable error upon starting.
pub async fn handle_srv(args: SrvArgs) -> Result<()> {
    info!("Handling srv command with args: {:?}", args);

    // Load configuration using the `config` submodule.
    // This merges command-line arguments with any potential configuration file settings.
    let config = config::load_and_merge_config(args).await?;
    info!("Effective server config: {:?}", config);

    // Run the server using the `server_logic` submodule.
    // This function contains the main server loop and request handling.
    server_logic::run_server(config).await?;

    // If the server logic completes without returning an error, indicate success.
    Ok(())
}
