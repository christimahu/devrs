//! # DevRS HTTP Server Implementation
//!
//! File: cli/src/commands/srv/server_logic.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the core HTTP server functionality for the `devrs srv`
//! command. It provides a robust static file server with features for local development:
//! - Static file serving with mime-type detection
//! - Port availability checking with automatic fallback
//! - CORS configuration
//! - Graceful shutdown handling
//!
//! ## Architecture
//!
//! The server implementation uses Axum and follows these steps:
//! 1. Set up the Axum router with appropriate middleware
//! 2. Find an available port if requested one is in use
//! 3. Start the server with graceful shutdown handlers
//! 4. Display connection information (URLs, etc.)
//!
//! ## Examples
//!
//! Basic usage from the command handler:
//!
//! ```rust
//! // Load configuration
//! let config = config::load_and_merge_config(args).await?;
//!
//! // Run the server
//! server_logic::run_server(config).await?;
//! ```
//!
//! The module provides a clean separation between configuration loading
//! and server implementation, allowing for flexible configuration through
//! command-line arguments and config files.
//!
use super::config::ServerConfig;
use super::utils;
use crate::core::error::Result;
use anyhow::Context;
use axum::Router; // Only Router needed from axum main crate directly
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    services::ServeDir, // Only ServeDir needed from services now
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};

use tracing::{error, info, warn, Level};

// ServerState struct is removed as it's no longer used.

/// # Run HTTP Server (`run_server`)
///
/// Initializes and starts the main Axum HTTP server according to the provided configuration.
///
/// ## Process:
/// 1. Determines an available network address (host and port) using `find_available_port`,
///    retrying if the initially configured port is occupied.
/// 2. Logs the contents of the directory being served (for debugging purposes).
/// 3. Retrieves the local network IP address for display.
/// 4. Creates the core Axum application router using `create_app`, which configures static file serving.
/// 5. Prints detailed server information to the console (serving directory, URLs, port, CORS status, etc.).
/// 6. Binds a `TcpListener` to the determined network address.
/// 7. Starts the Axum server, serving the application via the listener.
/// 8. Configures graceful shutdown handling using `shutdown_signal` to respond to Ctrl+C or termination signals.
///
/// ## Arguments
///
/// * `config`: The `ServerConfig` struct containing all necessary server settings (port, host, directory, CORS, etc.).
///
/// ## Returns
///
/// * `Result<()>`: Returns `Ok(())` if the server starts and runs successfully until shutdown.
///   Returns an `Err` if any critical step fails (e.g., cannot find an available port, listener binding fails, server error).
///
/// ## Errors
///
/// This function can return errors if:
/// - An available port cannot be found within the specified attempts.
/// - Binding the `TcpListener` to the chosen address fails (e.g., permissions).
/// - The Axum server itself encounters a fatal error during operation.
pub async fn run_server(config: ServerConfig) -> Result<()> {
    // Determine the final network address (host + port) to bind to.
    let max_port_attempts = 10;
    let addr = find_available_port(config.host, config.port, max_port_attempts).await?;

    // Log directory contents for debugging purposes (optional).
    utils::log_directory_contents(&config.directory);

    // Get a non-localhost IP address if available, for displaying network URLs.
    let local_ip = utils::get_local_ip();

    // Create the Axum application router with all routes and middleware.
    let app = create_app(&config); // Pass config by reference

    // Display detailed server information to the user upon startup.
    println!("\n=================================================================");
    println!("📂 Serving files from: {}", config.directory.display());
    println!("🌐 Local URL:         http://localhost:{}", addr.port());
    if local_ip != "localhost" {
        println!("🔗 Network URL:       http://{}:{}", local_ip, addr.port());
    }
    println!("⚙️  Binding to address: {}", addr);
    println!("❓ Index file:        {}", config.index_file); // Still relevant info even if fallback removed
    println!("🔒 CORS enabled:      {}", config.enable_cors);
    println!("👻 Show hidden files: {}", config.show_hidden);
    println!("=================================================================\n");

    // Log server startup details.
    info!(
        "Starting server on {} for directory {}",
        addr,
        config.directory.display()
    );
    println!("Server starting! Press Ctrl+C to stop.");

    // Bind the TCP listener to the determined socket address.
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind TCP listener to address {}", addr))?;

    // Start the Axum server, serving the application (`app`) using the listener.
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("HTTP server failed")?; // Propagate any server errors.

    println!("\nServer shutdown complete.");
    Ok(())
}

/// # Handle Shutdown Signal (`shutdown_signal`)
///
/// Creates a future that resolves when a shutdown signal (Ctrl+C or SIGTERM on Unix)
/// is received. This is used by `axum::serve`'s `with_graceful_shutdown` method
/// to allow the server to stop accepting new connections and finish processing
/// existing requests before exiting.
///
/// ## Returns
///
/// * `impl Future<Output = ()>`: A future that completes when either Ctrl+C is detected
///   or a SIGTERM signal is received (on Unix systems).
async fn shutdown_signal() {
    // Future that completes when Ctrl+C is pressed.
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        info!("Received Ctrl+C, initiating graceful shutdown...");
    };

    // Future that completes when SIGTERM is received (Unix-specific).
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut term) => {
                term.recv().await;
                info!("Received SIGTERM, initiating graceful shutdown...");
            }
            Err(e) => {
                error!(
                    "Failed to install SIGTERM handler: {}. Shutdown on SIGTERM might not work.",
                    e
                );
                // Keep the future pending indefinitely if the handler fails.
                std::future::pending::<()>().await;
            }
        }
    };

    // On non-Unix platforms, SIGTERM handling is not applicable, so create a future that never completes.
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    // Wait for either Ctrl+C or SIGTERM to occur.
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// # Find Available Port (`find_available_port`)
///
/// Attempts to find an available network port for the server to bind to. It starts
/// with the `start_port` and sequentially tries subsequent ports up to `max_attempts`
/// times if the initial port is already in use.
///
/// ## Arguments
///
/// * `req_host`: The `std::net::IpAddr` (host address) to attempt binding on.
/// * `start_port`: The initial port number to try binding.
/// * `max_attempts`: The maximum number of consecutive ports to try after `start_port` before giving up.
///
/// ## Returns
///
/// * `Result<SocketAddr>`: If an available port is found, returns `Ok` with the `SocketAddr` (combination of host and the available port).
///
/// ## Errors
///
/// Returns an `Err` if no available port is found within the allowed number of attempts,
/// wrapping the last encountered binding error or a custom error message.
async fn find_available_port(
    req_host: std::net::IpAddr,
    start_port: u16,
    max_attempts: u8,
) -> Result<SocketAddr> {
    let mut current_port = start_port;

    // Loop through ports sequentially, starting from start_port.
    for attempt in 0..max_attempts {
        let addr = SocketAddr::new(req_host, current_port);

        // Try to bind a listener to the current address.
        match TcpListener::bind(addr).await {
            Ok(listener) => {
                // Successfully bound! Drop the listener immediately to free the port.
                drop(listener);

                // Log if we had to use a different port than initially requested.
                if attempt > 0 {
                    info!(
                        "Port {} was unavailable, successfully bound to available port {}.",
                        start_port, current_port
                    );
                }

                // Return the available address.
                return Ok(addr);
            }
            Err(e) => {
                // Binding failed, likely because the port is in use.
                warn!(
                    "Attempt {}: Port {} on host {} is unavailable (Error: {}). Trying next port...",
                    attempt + 1,
                    current_port,
                    req_host,
                    e
                );
                // Increment port number for the next attempt.
                current_port += 1;
            }
        }
    }

    // If the loop finishes without finding a port, return an error.
    anyhow::bail!(
        "Could not find an available port on host {} starting from port {} after trying {} ports.",
        req_host,
        start_port,
        max_attempts
    )
}

/// # Create Axum Application (`create_app`)
///
/// Constructs and configures the main Axum `Router` instance, including middleware
/// (CORS, tracing) and the static file serving route. This version relies solely on
/// `ServeDir` to handle static file requests.
///
/// ## Arguments
///
/// * `config`: A reference to the `ServerConfig` containing settings like the serving directory,
/// index file name, and CORS enablement flag.
///
/// ## Returns
///
/// * `Router`: The fully configured Axum `Router` ready to be served.
fn create_app(config: &ServerConfig) -> Router {
    // Log a warning if --show-hidden is used, as its behavior may depend on ServeDir.
    if config.show_hidden {
        warn!(
            "--show-hidden flag is enabled but its behavior depends on the underlying file serving library"
        );
    }

    // Configure the CORS middleware layer based on the config flag.
    let cors_layer = if config.enable_cors {
        info!("CORS middleware enabled (permissive).");
        CorsLayer::permissive()
    } else {
        info!("CORS middleware disabled.");
        CorsLayer::new() // Effectively a no-op layer.
    };

    // Configure the tracing middleware for logging HTTP requests and responses.
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::default().include_headers(true))
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO));

    // Set up the static file service using ServeDir.
    // This service will handle GET requests for files within the configured directory.
    // Requests for non-existent files will result in a 404 response from ServeDir.
    let serve_dir_service = ServeDir::new(&config.directory);

    // Build the main router.
    Router::new()
        // Mount the ServeDir service at the root.
        // All GET requests will be handled by this service.
        .route_service("/", serve_dir_service)
        // Apply middleware layers.
        .layer(
            ServiceBuilder::new()
                .layer(trace_layer) // Apply tracing first.
                .layer(cors_layer), // Then apply CORS.
        )
}


// Fallback handler function is removed as it's no longer referenced.


// --- Unit Tests ---

/// # Unit Tests for Server Logic
///
/// Contains tests for individual functions within the `server_logic` module,
/// focusing on isolated functionality like port finding and router creation.
/// Full integration tests involving running the server are typically separate.
#[cfg(test)]
mod tests {
    use super::*; // Import items from the parent module (server_logic.rs).
    use std::net::Ipv4Addr;
    use tempfile::TempDir; // For creating temporary directories.
    use tokio::fs;          // For async file system operations in tests.

    /// Test finding an available port when the start port is free.
    #[tokio::test]
    async fn test_find_available_port_start_is_free() -> Result<()> {
        let host = Ipv4Addr::LOCALHOST.into();
        // Use a high port number likely to be free.
        let start_port = 50000;

        // Find an available port, expecting it to be the start_port.
        let addr = find_available_port(host, start_port, 5).await?;

        // Assert the found port is the one we started with.
        assert_eq!(addr.port(), start_port);
        assert_eq!(addr.ip(), host);

        Ok(())
    }

    /// Test finding an available port when the start port is occupied.
    #[tokio::test]
    async fn test_find_available_port_start_occupied() -> Result<()> {
        let host = Ipv4Addr::LOCALHOST.into();
        let start_port = 51000; // Start port for the test

        // Occupy the start_port
        let _listener = TcpListener::bind(SocketAddr::new(host, start_port)).await?;

        // Now try to find an available port starting from the occupied one.
        let addr = find_available_port(host, start_port, 5).await?;

        // The found port should be greater than the occupied start_port.
        assert!(addr.port() > start_port);
        assert_eq!(addr.ip(), host);
        // Ensure it didn't exceed the attempts range significantly (optional check)
        assert!(addr.port() < start_port + 5);

        // _listener is dropped here, releasing the initial port.
        Ok(())
    }

    /// Test the creation of the main Axum application router (simplified version).
    #[tokio::test]
    async fn test_create_app_builds() -> Result<()> {
        // Setup: Create a temporary directory for the test.
        let temp_dir = TempDir::new()?;
        let dir_path = temp_dir.path().to_path_buf();
        let index_file_name = "index.html"; // Name is still part of config

        // Setup: Create a dummy index file (though not directly used for fallback in this router).
        fs::write(dir_path.join(index_file_name), "<html>Test</html>")
            .await?;

        // Setup: Create a minimal ServerConfig pointing to the temp directory.
        let config = ServerConfig {
            port: 0, // Port doesn't matter for router creation itself.
            host: Ipv4Addr::LOCALHOST.into(),
            directory: dir_path.clone(), // Use the temp dir path
            enable_cors: true,           // Test with CORS enabled.
            show_hidden: false,
            index_file: index_file_name.to_string(),
        };

        // Action: Create the Axum app router.
        let app = create_app(&config);

        // Assert: Check if the app (Router) was created successfully.
        assert_ne!(format!("{:?}", app), ""); // Basic check that it's not empty/null.

        Ok(())
    }

    /// Test that the shutdown signal future can be created without panicking.
    #[tokio::test]
    async fn test_shutdown_signal_creation() {
        // This test primarily ensures the function compiles and doesn't panic
        // during setup, especially with platform-specific logic.
        let shutdown_future = shutdown_signal();
        // We don't await the future, just ensure its creation was successful.
        drop(shutdown_future);
        assert!(true); // Indicate success if no panic occurred.
    }
}
