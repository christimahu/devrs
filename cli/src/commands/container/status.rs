//! # DevRS Container Status Handler
//!
//! File: cli/src/commands/container/status.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs container status` subcommand, which shows
//! the status of application-specific Docker containers currently managed by Docker.
//! It provides a simplified view compared to `docker ps`, specifically filtering out
//! the core DevRS development environment container(s) to focus only on application containers.
//!
//! ## Architecture
//!
//! The implementation follows these steps:
//!
//! 1. Parse command-line arguments (`StatusArgs`), primarily the `--all` flag.
//! 2. Load the global DevRS configuration (`core::config`) to determine the naming pattern of the core development environment container(s) to exclude.
//! 3. Call the shared Docker utility `common::docker::state::list_containers` to get a list of all containers (running or all, based on the `--all` flag).
//! 4. Filter the raw list of containers, removing any whose names match the core DevRS environment pattern.
//! 5. If no application containers remain after filtering, print an appropriate message.
//! 6. If application containers are found, iterate through the filtered list and call `print_container_summary` for each one to display its details in a simple format.
//!
//! ## Usage
//!
//! ```bash
//! # Show running application containers (default)
//! devrs container status
//!
//! # Show all application containers (including stopped ones)
//! devrs container status --all
//! # Equivalent shorthand:
//! devrs container status -a
//! ```
//!
//! The output provides key details like Container ID (short), Image, Command, Created Timestamp, Status, Names, and Ports for each application container found.
//!
use crate::{
    common::docker, // Access shared Docker utilities (list_containers).
    core::{config, error::Result}, // Standard config loading and Result type.
};
use anyhow::Context; // For adding context to errors.
use clap::Parser; // For parsing command-line arguments.
use bollard::models::PortTypeEnum; // Enum used for formatting port protocol type.
use tracing::{debug, info}; // Logging framework utilities.

/// # Container Status Arguments (`StatusArgs`)
///
/// Defines the command-line arguments accepted by the `devrs container status` subcommand.
/// Uses the `clap` crate for parsing and validation.
#[derive(Parser, Debug)]
#[command(
    about = "Show status of application containers (excludes the core dev environment)",
    long_about = "Lists running (or all) application containers and their details.\n\
                  This command specifically excludes containers matching the core\n\
                  DevRS environment naming pattern defined in the configuration."
)]
pub struct StatusArgs {
    /// Optional: If set, displays all application containers, including those that are stopped.
    /// By default (if this flag is omitted), only currently running containers are shown.
    #[arg(long, short)] // Define as `--all` or `-a`.
    all: bool,
}

/// # Handle Container Status Command (`handle_status`)
///
/// The main asynchronous handler function for the `devrs container status` command.
/// It retrieves a list of Docker containers, filters out the core DevRS environment
/// container(s), and prints a summary of the remaining application containers.
///
/// ## Workflow:
/// 1.  Logs the command execution and the value of the `all` flag.
/// 2.  Loads the DevRS configuration to get the `core_env.image_name`, which is used to construct the naming pattern (`<image_name>-*`) for the core environment container(s) that should be excluded from the listing.
/// 3.  Calls `common::docker::state::list_containers`, passing the `args.all` flag to determine whether to list only running or all containers.
/// 4.  Filters the returned list of `ContainerSummary` objects: it keeps only those containers whose names *do not* start with the core environment pattern (after stripping the leading '/').
/// 5.  Checks if the filtered list (`app_containers`) is empty.
///     * If empty, prints a "No application containers found" message (adjusting the message based on the `all` flag).
///     * If not empty, iterates through the `app_containers` vector and calls `print_container_summary` for each one, followed by a separator line. Finally, prints a summary count.
///
/// ## Arguments
///
/// * `args`: The parsed `StatusArgs` struct containing the `all` flag.
///
/// ## Returns
///
/// * `Result<()>`: Returns `Ok(())` if the container list was successfully retrieved and displayed.
/// * `Err`: Returns an `Err` if configuration loading or the Docker API call to list containers fails.
pub async fn handle_status(args: StatusArgs) -> Result<()> {
    // Log entry point and arguments.
    info!("Handling container status command (All: {})", args.all);

    // Load config to identify the core dev environment container pattern to exclude.
    let cfg = config::load_config().context("Failed to load DevRS configuration")?;
    let core_env_image_name = &cfg.core_env.image_name;
    // Define the pattern based on the configured image name. Assumes core env container names start with this.
    let core_env_name_pattern = format!("{}-*", core_env_image_name);
    debug!(
        "Excluding containers matching pattern: {}",
        core_env_name_pattern
    ); // Log the exclusion pattern.

    // Get the list of containers from Docker. `args.all` determines if stopped containers are included.
    let container_summaries = docker::state::list_containers(args.all, None) // Pass None for filters; filtering is done locally.
        .await // Await the async Docker API call.
        .context("Failed to list Docker containers")?; // Add context on error.

    // Filter the list to exclude containers matching the core dev environment pattern.
    let app_containers: Vec<_> = container_summaries
        .into_iter() // Consume the original vector.
        .filter(|c| { // Keep container 'c' if the closure returns true.
            // Check if *any* of the container's names match the exclusion pattern.
            !c.names.as_ref() // Get Option<&Vec<String>> for names.
                .map_or(false, |names| { // If names exist...
                    names.iter().any(|name| { // Check if any name matches...
                        let clean_name = name.trim_start_matches('/'); // Docker often prefixes names with '/'.
                        // Check if the cleaned name starts with the core env pattern prefix.
                        clean_name.starts_with(&core_env_name_pattern.trim_end_matches('*'))
                    })
                })
        })
        .collect(); // Collect the filtered containers into a new vector.

    // --- Print Results ---
    // Check if any application containers were found after filtering.
    if app_containers.is_empty() {
        // Print message indicating none were found.
        println!(
            "No application containers found{}.",
            if args.all { "" } else { " (running)" } // Adjust message based on --all flag.
        );
        // Suggest using --all if only running containers were checked.
        if !args.all {
            println!("Try running with --all to include stopped containers.");
        }
    } else {
        // Print a header for the list.
        println!("\n--- Application Containers ---");
        // Iterate through the filtered application containers.
        for container in &app_containers {
            // Print a formatted summary for each container.
            print_container_summary(container);
            // Print a separator line between container summaries.
            println!("--------------------");
        }
        // Print a count of the containers found.
        println!("Found {} application container(s).", app_containers.len());
    }

    Ok(()) // Indicate successful command execution.
}

/// # Print Container Summary (`print_container_summary`)
///
/// Prints a simple, formatted summary of a single container's details to stdout.
/// This function takes information from the `ContainerSummary` struct provided by the
/// Docker API (via `bollard`) and displays key fields in a readable format.
///
/// ## Arguments
///
/// * `container`: A reference (`&`) to the `bollard::models::ContainerSummary` struct containing the container's details.
fn print_container_summary(container: &bollard::models::ContainerSummary) {
    // Extract and format key information, providing "N/A" or defaults if fields are missing.

    // Get the short container ID (first 12 characters).
    let id = container.id.as_deref().unwrap_or("N/A")[..12].to_string();
    // Get the image name.
    let image = container.image.as_deref().unwrap_or("N/A");
    // Get the command run by the container.
    let command = container.command.as_deref().unwrap_or("N/A");
    // Get the human-readable status string (e.g., "Up 2 hours", "Exited (0) 5 minutes ago").
    let status = container.status.as_deref().unwrap_or("N/A");
    // Get the list of names assigned to the container, format them nicely.
    let names_str = container.names.as_ref().map_or("N/A".to_string(), |names| {
        names
            .iter()
            // Remove the leading '/' often added by Docker.
            .map(|n| n.trim_start_matches('/'))
            // Join multiple names with ", ".
            .collect::<Vec<_>>()
            .join(", ")
    });

    // Get the creation timestamp (as a Unix timestamp integer).
    // Note: Displaying this as a human-readable date/time requires the `chrono` crate,
    // which was removed for simplicity here. We just show the raw timestamp.
    let created_str = container
        .created // This is an Option<i64>.
        .map_or("N/A".to_string(), |ts| ts.to_string()); // Convert timestamp to string if present.

    // Print the extracted information with labels.
    println!("ID:      {}", id);
    println!("Image:   {}", image);
    println!("Command: {}", command);
    println!("Created: {} (timestamp)", created_str); // Clarify that 'Created' is a timestamp.
    println!("Status:  {}", status);
    println!("Names:   {}", names_str);

    // --- Print Port Mappings ---
    // Check if port information is available.
    if let Some(ports) = &container.ports {
        if ports.is_empty() {
            // No ports exposed or mapped.
            println!("Ports:   <none>");
        } else {
            // Print each port mapping.
            println!("Ports:");
            for p in ports {
                // Determine the protocol string (e.g., "/tcp", "/udp").
                let typ_str = match &p.typ {
                    Some(PortTypeEnum::TCP) => "/tcp",
                    Some(PortTypeEnum::UDP) => "/udp",
                    Some(PortTypeEnum::SCTP) => "/sctp", // Less common but possible.
                    _ => "", // Default to empty if type is unknown or missing.
                };
                // Format the mapping string: HOST_IP:HOST_PORT -> CONTAINER_PORT/PROTOCOL
                let port_mapping = format!(
                    "  - {}:{}{}->{}{}", // Template string.
                    p.ip.as_deref().unwrap_or("0.0.0.0"), // Host IP (defaults to 0.0.0.0 if not specified).
                    // Public (host) port - handle case where it might not be mapped.
                    p.public_port
                        .map_or_else(|| "<none>".to_string(), |pp| pp.to_string()),
                    typ_str, // Protocol associated with host port mapping.
                    p.private_port, // Container's internal port (u16).
                    typ_str // Protocol associated with container port.
                );
                println!("{}", port_mapping); // Print the formatted line.
            }
        }
    } else {
        // Port information was entirely absent in the summary.
        println!("Ports:   <none>");
    }
}


// --- Unit Tests ---
// Focus on argument parsing and the formatting logic of `print_container_summary`.
// Testing `handle_status` requires mocking config and Docker API calls.
#[cfg(test)]
mod tests {
    use super::*;
    // Import necessary structs from bollard models for creating test data.
    use bollard::models::{ContainerStateStatusEnum, ContainerSummary, Port, PortTypeEnum};

    /// Test argument parsing for the default case (no flags).
    #[test]
    fn test_status_args_parsing() {
        // Simulate `devrs container status`
        let args = StatusArgs::try_parse_from(&["status"]).unwrap();
        // Default value for `all` should be false.
        assert!(!args.all);
    }

    /// Test argument parsing with the `--all` flag (or `-a`).
    #[test]
    fn test_status_args_parsing_all() {
         // Simulate `devrs container status -a`
        let args_all = StatusArgs::try_parse_from(&["status", "-a"]).unwrap();
        // The `all` flag should be true.
        assert!(args_all.all);
    }

    /// Test the formatting logic of the `print_container_summary` helper function.
    /// This creates a mock `ContainerSummary` and calls the print function.
    /// The main goal is to ensure it runs without panicking and produces some output.
    /// Visual inspection of test output (`cargo test -- --nocapture`) can verify formatting.
    #[test]
    fn test_print_container_summary_formatting() {
        // Create a sample ContainerSummary with various fields populated.
        let container = ContainerSummary {
            id: Some("1234567890abcdef".to_string()), // Example long ID.
            names: Some(vec!["/my-app-1".to_string(), "/secondary_name".to_string()]), // Multiple names.
            image: Some("myapp:latest".to_string()),
            command: Some("/app/run -p 80 --verbose".to_string()), // Example command.
            created: Some(1678886400), // Example Unix timestamp.
            status: Some("Up About an hour".to_string()), // Example status string.
            state: Some(ContainerStateStatusEnum::RUNNING.to_string()), // Example state.
            ports: Some(vec![
                // Mapped TCP port.
                Port {
                    private_port: 80, // Container port.
                    public_port: Some(8080), // Host port.
                    typ: Some(PortTypeEnum::TCP), // Protocol.
                    ip: Some("0.0.0.0".to_string()), // Host IP.
                },
                // Exposed but unmapped port.
                Port {
                    private_port: 9000,
                    public_port: None, // No host port mapping.
                    typ: Some(PortTypeEnum::TCP),
                    ip: None, // No specific host IP.
                },
                 // Mapped UDP port.
                 Port {
                    private_port: 53,
                    public_port: Some(10053),
                    typ: Some(PortTypeEnum::UDP), // UDP protocol.
                    ip: Some("127.0.0.1".to_string()), // Mapped only to localhost.
                },
            ]),
            ..Default::default() // Use default values for other fields.
        };

        // Call the function with the mock data.
        // This test primarily checks that the function executes without panicking
        // due to missing fields or formatting issues.
        println!("--- Start print_container_summary Output ---");
        print_container_summary(&container);
        println!("--- End print_container_summary Output ---");
        // Assertions could be added here if stdout capture was implemented,
        // but for now, successful execution without panic is the main check.
    }

    // Note: Integration tests for `handle_status` would require mocking:
    // 1. `config::load_config` to provide a known core env image name pattern.
    // 2. `docker::state::list_containers` to return a controlled list of `ContainerSummary` objects,
    //    including some that should be filtered out and some that should be displayed.
    // Then, stdout capture could be used to verify the final printed output.
}
