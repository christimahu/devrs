//! # DevRS Environment Status Handler
//!
//! File: cli/src/commands/env/status.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs env status` subcommand. Its purpose is to
//! display detailed status information about the **core development environment**
//! container. This includes its running state, image details, network settings
//! (IP, ports), and configured mounts.
//!
//! ## Architecture
//!
//! The command execution follows these steps:
//! 1. Parse command-line arguments (`StatusArgs`) using `clap`, specifically the optional `--name` override.
//! 2. Load the DevRS configuration (`core::config`) to get core environment settings (image name/tag, default container name).
//! 3. Determine the target core environment container name (using `--name` or the default derived from config).
//! 4. Check if the configured core environment *image* exists locally using `common::docker::images::image_exists`, logging a warning if not found.
//! 5. Attempt to inspect the target *container* using `common::docker::state::inspect_container`.
//! 6. If inspection is successful, pass the detailed container information (`ContainerInspectResponse`) to the `print_container_details` function for formatted output.
//! 7. If inspection fails with a `ContainerNotFound` error, print a user-friendly message indicating the container doesn't exist and suggest next steps (`devrs env shell` or `devrs env build`). Treat this as a successful command execution (the status *is* "not found").
//! 8. If inspection fails with any other error, propagate it up the call stack.
//!
//! ## Usage
//!
//! ```bash
//! # Show status of the default core environment container
//! devrs env status
//!
//! # Show status of a specifically named core environment container
//! devrs env status --name my-custom-env-instance
//! ```
//!
//! The output provides a comprehensive overview of the core environment's current state.
//!
use crate::{
    common::docker::{self}, // Access shared Docker utilities (image_exists, inspect_container).
    core::{
        config,                      // Access configuration loading.
        error::{DevrsError, Result}, // Standard Result type and custom errors.
    },
};
use anyhow::Context; // For adding context to errors.
use chrono::{DateTime, Local}; // For formatting timestamps nicely. Using Local time zone.
use clap::Parser; // For parsing command-line arguments.
use tracing::{debug, info, warn}; // Logging framework utilities.

/// # Environment Status Arguments (`StatusArgs`)
/// Defines the command-line arguments accepted by the `devrs env status` subcommand.
/// Uses the `clap` crate for parsing and validation.
#[derive(Parser, Debug)]
#[command(about = "Show status of the core development environment container")]
pub struct StatusArgs {
    /// Optional: Specifies the exact name of the core environment container to check the status of.
    /// If omitted, the default name (derived from the `core_env.image_name` in the configuration,
    /// typically `<image_name>-instance`) is used.
    #[arg(long)] // Defines the `--name <NAME>` option.
    name: Option<String>,
    // TODO: Consider adding flags like `--json` in the future for machine-readable output.
}

/// # Handle Environment Status Command (`handle_status`)
/// The main asynchronous handler function for the `devrs env status` command.
/// It retrieves and displays detailed information about the core development environment container.
///
/// ## Workflow:
/// 1. Logs the start and parsed arguments.
/// 2. Loads the DevRS configuration.
/// 3. Determines the target container name (using `--name` or default).
/// 4. Checks if the configured core environment *image* exists locally, logging a warning if not.
/// 5. Calls `common::docker::state::inspect_container` to get detailed information about the target container.
/// 6. Processes the result of the inspection:
///    - If `Ok(details)`, calls `print_container_details` to display the information.
///    - If `Err` is `DevrsError::ContainerNotFound`, prints a helpful "not found" message and returns `Ok(())`.
///    - If any other `Err` occurs, propagates the error.
///
/// ## Arguments
/// * `args`: The parsed `StatusArgs` struct containing the optional container `name`.
///
/// ## Returns
/// * `Result<()>`: `Ok(())` if the status was successfully retrieved and displayed (including the "not found" state).
/// * `Err`: If config loading fails or a non-"not found" Docker API error occurs during inspection.
pub async fn handle_status(args: StatusArgs) -> Result<()> {
    info!("Handling env status command..."); // Log entry.
    debug!("Status args: {:?}", args); // Log arguments if debug enabled.

    // 1. Load configuration.
    let cfg = config::load_config().context("Failed to load DevRS configuration")?;

    // 2. Determine the target container name.
    let container_name = args.name.clone().unwrap_or_else(|| {
        // Use helper to get default name if --name not provided.
        let default_name = get_core_env_container_name(&cfg);
        debug!("No specific name provided, using default: {}", default_name);
        default_name
    });

    // 3. Check if the *image* for the core environment exists locally (informational).
    let image_name_with_tag = format!("{}:{}", cfg.core_env.image_name, cfg.core_env.image_tag);
    match docker::images::image_exists(&image_name_with_tag).await {
        //
        Ok(true) => info!(
            // Log if image found.
            "Core environment image '{}' found locally.",
            image_name_with_tag
        ),
        Ok(false) => warn!(
            // Warn if image not found.
            "Core environment image '{}' not found locally. Run 'devrs env build'.",
            image_name_with_tag
        ),
        Err(e) => warn!("Could not check image existence: {}", e), // Warn on error checking image.
    }

    println!(
        // Inform user which container status is being checked.
        "Checking status for core environment container '{}'...",
        container_name
    );

    // 4. Inspect the target container.
    match docker::state::inspect_container(&container_name).await {
        //
        Ok(details) => {
            // 5. If inspection succeeded, print the details.
            print_container_details(&container_name, &details);
        }
        Err(e) => {
            // Check if the error was specifically 'ContainerNotFound'.
            if e.downcast_ref::<DevrsError>()
                .is_some_and(|de| matches!(de, DevrsError::ContainerNotFound { .. }))
            {
                // Container doesn't exist - print a helpful message.
                println!("\nStatus: Container '{}' not found.", container_name);
                println!("Run 'devrs env shell' to create and start it, or 'devrs env build' if the image is missing.");
                // Return Ok here because displaying "not found" is considered successful execution of the status command.
                return Ok(());
            } else {
                // For any other type of error during inspection, propagate it.
                return Err(e).context(format!("Failed to inspect container '{}'", container_name));
            }
        }
    }

    Ok(()) // Overall command success.
}

/// # Get Core Environment Container Name (`get_core_env_container_name`)
/// Helper function to derive the default container name based on the configured image name.
/// Appends "-instance" to the image name.
fn get_core_env_container_name(cfg: &config::Config) -> String {
    // Simple string formatting to construct the default name.
    format!("{}-instance", cfg.core_env.image_name)
}

/// # Print Container Details (`print_container_details`)
/// Formats and prints detailed information about a container based on the inspection results
/// provided by the Docker API (`ContainerInspectResponse`).
///
/// ## Arguments
/// * `name`: The name of the container being displayed (used for the header).
/// * `details`: A reference (`&`) to the `bollard::models::ContainerInspectResponse` struct containing the raw details from Docker.
fn print_container_details(name: &str, details: &bollard::models::ContainerInspectResponse) {
    // --- Extract Key Sections from Details ---
    // Safely access optional fields within the response struct.
    let state = details.state.as_ref();
    let config = details.config.as_ref();
    let network_settings = details.network_settings.as_ref();

    // --- Print Header ---
    println!("\n--- Core Environment Status: {} ---", name);

    // --- Basic Info ---
    println!("  ID:          {}", details.id.as_deref().unwrap_or("N/A")); // Print short ID.
    println!(
        "  Image:       {}",
        config.and_then(|c| c.image.as_deref()).unwrap_or("N/A") // Get image name from config section.
    );

    // --- State Info ---
    println!(
        "  Status:      {}",
        // Extract the status enum (e.g., RUNNING, EXITED), format it if present.
        state
            .and_then(|s| s.status.as_ref().map(|st| format!("{:?}", st))) // Use Debug format for the enum.
            .unwrap_or_else(|| "Unknown".to_string()) // Fallback if status is missing.
    );
    println!(
        "  Running:     {}",
        // Extract the boolean 'running' flag.
        state
            .and_then(|s| s.running)
            .map_or("Unknown", |r| if r { "Yes" } else { "No" }) // Convert bool to Yes/No string.
    );
    // Format and print 'StartedAt' timestamp in local timezone.
    if let Some(started_at_str) = state.and_then(|s| s.started_at.as_deref()) {
        match DateTime::parse_from_rfc3339(started_at_str) {
            // Parse RFC3339 timestamp.
            Ok(dt) => println!(
                // Format into local timezone YYYY-MM-DD HH:MM:SS ZONE.
                "  Started At:  {}",
                dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S %Z")
            ),
            Err(_) => println!("  Started At:  {} (Could not parse)", started_at_str), // Handle parse failure.
        }
    }
    // Format and print 'FinishedAt' timestamp if the container has exited.
    if let Some(finished_at_str) = state.and_then(|s| s.finished_at.as_deref()) {
        // Docker often uses a zero-value timestamp for non-exited containers; ignore those.
        if !finished_at_str.starts_with("0001-01-01") {
            match DateTime::parse_from_rfc3339(finished_at_str) {
                Ok(dt) => println!(
                    "  Finished At: {}",
                    dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S %Z")
                ),
                Err(_) => println!("  Finished At: {} (Could not parse)", finished_at_str),
            }
        }
    }
    // Print exit code if available.
    if let Some(exit_code) = state.and_then(|s| s.exit_code) {
        println!("  Exit Code:   {}", exit_code);
    }

    // --- Network Info ---
    println!("  Network:");
    // Try getting the primary IP address directly.
    let ip_address = network_settings
        .and_then(|ns| ns.ip_address.as_deref())
        .filter(|ip| !ip.is_empty()); // Ignore if empty string.

    if let Some(ip) = ip_address {
        println!("    IP Address:  {}", ip);
    } else {
        // Fallback: Check the IP address within the attached networks (common for bridge network).
        if let Some(ip) = network_settings
            .and_then(|ns| ns.networks.as_ref()) // Get Option<&HashMap<String, EndpointSettings>>
            .and_then(|nets| nets.values().next()) // Get details of the first network found.
            .and_then(|ep| ep.ip_address.as_deref()) // Get IP from endpoint settings.
            .filter(|ip| !ip.is_empty())
        // Ignore if empty.
        {
            println!("    IP Address:  {} (from bridge network)", ip); // Indicate source.
        } else {
            // If no IP found either way.
            println!("    IP Address:  N/A (Container might be stopped or network not ready)");
        }
    }

    // Print Port Mappings.
    let ports = network_settings.and_then(|ns| ns.ports.as_ref()); // Option<&PortMap>
    if let Some(port_map) = ports {
        if port_map.is_empty() {
            println!("    Ports:       <none exposed/mapped>");
        } else {
            println!("    Ports:");
            // Sort ports by container port number for consistent output.
            let mut sorted_ports: Vec<_> = port_map.iter().collect();
            sorted_ports.sort_by_key(|(k, _)| {
                // k is "port/proto" string (e.g., "80/tcp")
                k.split('/') // Split into port and proto.
                    .next() // Get the port part.
                    .unwrap_or("0") // Default to 0 if split fails.
                    .parse::<u16>() // Parse as u16.
                    .unwrap_or(0) // Default to 0 on parse error.
            });

            // Iterate through sorted ports.
            for (container_port_proto, host_bindings) in sorted_ports {
                // `host_bindings` is Option<Vec<PortBinding>>.
                if let Some(bindings) = host_bindings {
                    if bindings.is_empty() {
                        // Port is exposed but not mapped to any host port.
                        println!("      - {} -> <not mapped>", container_port_proto);
                    } else {
                        // Port is mapped; print each host binding.
                        for binding in bindings {
                            println!(
                                "      - {} -> {}:{}",
                                container_port_proto, // e.g., "80/tcp"
                                binding.host_ip.as_deref().unwrap_or("0.0.0.0"), // Host IP (often 0.0.0.0).
                                binding.host_port.as_deref().unwrap_or("<none>")  // Host port.
                            );
                        }
                    }
                } else {
                    // Port is exposed but not mapped (represented as null in JSON).
                    println!("      - {} -> <not mapped>", container_port_proto);
                }
            }
        }
    } else {
        // Port information section was absent in the inspect response.
        println!("    Ports:       <none exposed/mapped>");
    }

    // --- Mounts Info ---
    // Access the mounts list (Option<Vec<MountPoint>>).
    let mounts = details.mounts.as_ref();
    if let Some(mount_list) = mounts {
        if mount_list.is_empty() {
            println!("  Mounts:      <none>");
        } else {
            println!("  Mounts:");
            // Clone and sort mounts by destination path for consistent output.
            let mut sorted_mounts = mount_list.clone();
            sorted_mounts.sort_by_key(|m| m.destination.clone()); // Sort by Option<String> destination.

            // Print details for each mount.
            for mount in &sorted_mounts {
                let dest = mount.destination.as_deref().unwrap_or("N/A"); // Container path.
                let src = mount.source.as_deref().unwrap_or("N/A"); // Host path or volume name.
                let mode = mount.rw.map_or("?", |rw| if rw { "rw" } else { "ro" }); // Read/write mode.
                                                                                    // Type (e.g., BIND, VOLUME) - format the enum if present.
                let typ = mount
                    .typ // Option<MountPointTypeEnum>
                    .as_ref()
                    .map_or("?".to_string(), |t| format!("{:?}", t)); // Use Debug format for the enum.
                println!("    - {} -> {} ({}, {})", src, dest, mode, typ);
            }
        }
    } else {
        // Mounts section was absent in the inspect response.
        println!("  Mounts:      <none>");
    }

    // --- Print Footer ---
    println!("-----------------------------------");
}

// --- Unit Tests ---
// Focus on argument parsing and the formatting logic of `print_container_details`.
// Testing `handle_status` requires mocking config and Docker API calls.
#[cfg(test)]
mod tests {
    use super::*;
    // Import necessary structs from bollard models for creating test data.
    use bollard::models::{
        ContainerConfig, ContainerInspectResponse, ContainerState, EndpointSettings, MountPoint,
        MountPointTypeEnum, NetworkSettings, PortBinding, PortMap,
    };
    use std::collections::HashMap; // For network settings map.

    /// Test parsing default arguments (no flags).
    #[test]
    fn test_status_args_parsing() {
        // Simulate `devrs env status`
        let args = StatusArgs::try_parse_from(["status"]).unwrap();
        // The optional `--name` should be None by default.
        assert!(args.name.is_none());
    }

    /// Test parsing with the optional `--name` flag.
    #[test]
    fn test_status_args_parsing_with_name() {
        // Simulate `devrs env status --name custom-env`
        let args_named = StatusArgs::try_parse_from(["status", "--name", "custom-env"]).unwrap();
        // The name should be parsed correctly.
        assert_eq!(args_named.name, Some("custom-env".to_string()));
    }

    /// Test the formatting logic of the `print_container_details` helper function.
    /// This creates a mock `ContainerInspectResponse` and calls the print function.
    /// The main goal is to ensure it runs without panicking and produces some output
    /// covering various states and optional fields.
    #[test]
    fn test_print_container_details_formatting() {
        // --- Create Mock Data ---
        let mock_state = Some(ContainerState {
            status: Some(bollard::models::ContainerStateStatusEnum::RUNNING),
            running: Some(true),
            paused: Some(false),
            restarting: Some(false),
            oom_killed: Some(false),
            dead: Some(false),
            pid: Some(1234),
            exit_code: Some(0), // Usually 0 if running
            error: Some("".to_string()),
            started_at: Some("2023-10-27T10:00:00.123456789Z".to_string()), // RFC3339 format
            finished_at: Some("0001-01-01T00:00:00Z".to_string()),          // Zero time if running
            health: None, // Omit health details for simplicity
        });

        let mock_config = Some(ContainerConfig {
            image: Some("test-image:latest".to_string()),
            // Add other fields as needed for testing specific output
            ..Default::default()
        });

        // Mock network settings with IP and ports
        let mut port_bindings = PortMap::new();
        port_bindings.insert(
            "80/tcp".to_string(),
            Some(vec![PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some("8080".to_string()),
            }]),
        );
        port_bindings.insert("443/tcp".to_string(), None); // Exposed but not mapped
        port_bindings.insert(
            "53/udp".to_string(),
            Some(vec![PortBinding {
                // UDP example
                host_ip: Some("127.0.0.1".to_string()),
                host_port: Some("53".to_string()),
            }]),
        );

        let mut networks = HashMap::new();
        networks.insert(
            "bridge".to_string(),
            EndpointSettings {
                ip_address: Some("172.17.0.2".to_string()), // Example bridge IP
                // Add other endpoint settings if needed
                ..Default::default()
            },
        );

        let mock_network = Some(NetworkSettings {
            // ip_address is often empty when using bridge network, test the fallback
            ip_address: Some("".to_string()),
            ports: Some(port_bindings),
            networks: Some(networks),
            ..Default::default()
        });

        // Mock mount points
        let mock_mounts = Some(vec![
            MountPoint {
                typ: Some(MountPointTypeEnum::BIND),
                source: Some("/home/user/code".to_string()),
                destination: Some("/app/code".to_string()),
                mode: Some("rw".to_string()), // Mode string
                rw: Some(true),               // Boolean RW flag
                ..Default::default()
            },
            MountPoint {
                typ: Some(MountPointTypeEnum::VOLUME),
                name: Some("my-data-volume".to_string()), // Volume name
                destination: Some("/data".to_string()),
                driver: Some("local".to_string()),
                mode: Some("rw".to_string()),
                rw: Some(true),
                ..Default::default()
            },
            MountPoint {
                // Read-only example
                typ: Some(MountPointTypeEnum::BIND),
                source: Some("/etc/config.conf".to_string()),
                destination: Some("/app/config.conf".to_string()),
                mode: Some("ro".to_string()),
                rw: Some(false),
                ..Default::default()
            },
        ]);

        // Assemble the mock ContainerInspectResponse
        let mock_details = ContainerInspectResponse {
            id: Some("abc123def456".to_string()),
            created: Some("2023-10-27T09:59:00.987654321Z".to_string()),
            name: Some("/test-container-name".to_string()),
            state: mock_state,
            config: mock_config,
            network_settings: mock_network,
            mounts: mock_mounts,
            // Fill other fields with Default::default() as needed
            ..Default::default()
        };

        // --- Call the function ---
        // This primarily checks that formatting different fields (including optional ones,
        // timestamps, enums, maps, vectors) doesn't cause panics.
        println!("--- Start print_container_details Output ---");
        print_container_details("test-container-name", &mock_details);
        println!("--- End print_container_details Output ---");
        // Visual inspection of test output (`cargo test -- --nocapture`) is needed
        // to fully verify the output format. Assertions could be added with stdout capture.
    }

    // Note: Integration tests for `handle_status` would require mocking:
    // 1. `config::load_config` -> To provide a known core env image name pattern.
    // 2. `common::docker::images::image_exists` -> To simulate image presence/absence.
    // 3. `common::docker::state::inspect_container` -> To return mock `ContainerInspectResponse` data
    //    or specific errors like `ContainerNotFound`.
    // Then, stdout capture could verify the final formatted output or the "not found" message.
}
