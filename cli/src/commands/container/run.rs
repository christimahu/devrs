//! # DevRS Container Run Command
//!
//! File: cli/src/commands/container/run.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs container run` command, which creates and
//! starts new application-specific Docker containers from specified images.
//! It provides a user-friendly interface over the core Docker run functionality,
//! allowing configuration of names, ports, environment variables, detached mode,
//! auto-removal, and command overrides.
//!
//! ## Architecture
//!
//! The command flow follows these steps:
//! 1. Parse command-line arguments (`RunArgs`) using `clap`.
//! 2. Load global DevRS configuration (`core::config`) to get potential defaults (like image prefix).
//! 3. Determine the final image name to use (prioritizing `--image`, then generating a default based on directory name and config prefix).
//! 4. Check if the target image exists locally using `common::docker::image_exists` (issues a warning if not found, Docker will attempt to pull).
//! 5. Determine the container name (prioritizing `--name`, then generating a default `devrs-app-<dirname>`).
//! 6. Parse environment variables provided via `--env KEY=VALUE` into a HashMap.
//! 7. Prepare port mappings from `--port HOST:CONTAINER` arguments.
//! 8. Prepare volume mount configurations (currently none supported directly via CLI args for this command).
//! 9. Prepare any command override provided as trailing arguments.
//!10. Call the shared Docker utility function `common::docker::run_container` with all prepared options.
//!11. Report success, indicating whether the container started in detached mode or finished running (for foreground mode).
//!
//! ## Examples
//!
//! Usage examples:
//!
//! ```bash
//! # Run using default image/container names based on current dir
//! devrs container run
//!
//! # Run with explicit image name
//! devrs container run --image myapp:1.0
//!
//! # Run with port mappings and environment variables
//! devrs container run --image myapp:1.0 -p 8080:80 -p 8443:443 -e KEY=VALUE
//!
//! # Run in detached mode with a specific name
//! devrs container run --image myapp:1.0 --name myapp-instance --detach
//!
//! # Run with auto-removal when finished (e.g., for a batch job)
//! devrs container run --image my-batch-job:latest --rm
//!
//! # Override the default command defined in the image
//! devrs container run --image my-base-image:latest /app/custom_entrypoint --config /etc/app.conf
//! ```
//!
//! The command handles both background (detached) and foreground container execution.
//! Note that foreground execution currently doesn't stream logs back interactively in this implementation;
//! it waits for the container process to complete. For interactive sessions, use `devrs container shell`.
//!
use crate::common::docker; // Access shared Docker utilities (run_container, image_exists).
use crate::core::config; // Access configuration loading.
use crate::core::error::Result; // Standard Result type for error handling.
use anyhow::Context; // For adding context to errors.
use clap::Parser; // For parsing command-line arguments.
use std::collections::HashMap; // Required for storing parsed environment variables.
use std::env; // For getting the current working directory.
use tracing::{debug, info, warn}; // Logging framework utilities.

/// # Container Run Arguments (`RunArgs`)
///
/// Defines the command-line arguments accepted by the `devrs container run` subcommand.
/// Uses the `clap` crate for parsing and validation.
#[derive(Parser, Debug)]
pub struct RunArgs {
    /// Optional: Specifies the name and tag of the Docker image to use for creating the container (e.g., "myapp:latest").
    /// If omitted, a default image name is generated based on the current directory name and an optional
    /// prefix from the DevRS configuration (format: `<prefix>-<dirname>:latest`).
    #[arg(long, short)] // Define as `--image` or `-i`
    pub image: Option<String>,

    /// Optional: Assigns a specific name to the newly created container.
    /// If omitted, a default name is generated (format: `devrs-app-<dirname>`).
    /// Container names must be unique across the Docker daemon.
    #[arg(long)] // Define as `--name`
    pub name: Option<String>,

    /// Optional: Specifies port mappings between the host and the container.
    /// Format: `HOST_PORT:CONTAINER_PORT` (e.g., "8080:80", "127.0.0.1:9000:9000").
    /// Can be specified multiple times to map multiple ports.
    #[arg(short, long = "port", action = clap::ArgAction::Append)] // Define as `-p` or `--port`, allowing multiple occurrences.
    pub ports: Vec<String>,

    /// Optional: Sets environment variables inside the container.
    /// Format: `KEY=VALUE` (e.g., "DATABASE_URL=postgres://...", "API_KEY=123").
    /// Can be specified multiple times to set multiple variables.
    #[arg(short, long = "env", action = clap::ArgAction::Append)] // Define as `-e` or `--env`, allowing multiple occurrences.
    pub env_vars: Vec<String>,

    /// Optional: Runs the container in the background (detached mode).
    /// If not set, the command will typically wait for the container's main process to finish
    /// (although this implementation might not currently stream foreground logs effectively).
    #[arg(short, long)] // Define as `--detach` or `-d`
    pub detach: bool,

    /// Optional: Automatically removes the container filesystem when the container exits.
    /// This is useful for short-lived containers or tasks where persistent state is not needed.
    /// Cannot typically be used with `--detach`.
    #[arg(long)] // Define as `--rm`
    pub rm: bool,

    /// Optional: Specifies a command and its arguments to run inside the container, overriding
    /// the default `CMD` or `ENTRYPOINT` defined in the image's Dockerfile.
    /// All arguments following the options are captured as the command.
    /// Example: `devrs container run --image alpine:latest echo "Hello from container"`
    #[arg(last = true)] // Capture all remaining arguments after options.
    pub command: Vec<String>,
    // TODO: Add volume mount (-v) arguments later.
}

/// # Handle Container Run Command (`handle_run`)
///
/// The main asynchronous handler function for the `devrs container run` command.
/// It takes the parsed arguments, determines image and container names, prepares options
/// like ports and environment variables, and then calls the shared Docker utility function
/// to create and start the container.
///
/// ## Workflow:
/// 1.  Logs the start and parsed arguments.
/// 2.  Loads global DevRS config to check for default image prefix.
/// 3.  Determines the target image name (using `--image` or generating default).
/// 4.  Checks if the image exists locally (logs warning if not).
/// 5.  Determines the target container name (using `--name` or generating default).
/// 6.  Parses `--env` arguments into a HashMap.
/// 7.  Prepares port mappings from `--port` arguments (passed as `Vec<String>`).
/// 8.  Prepares volume mounts (currently none are passed from CLI args).
/// 9.  Prepares the command override (`Option<Vec<String>>`) if provided.
/// 10. Calls `common::docker::run_container` with all prepared arguments.
/// 11. Reports success, distinguishing between detached start and foreground completion.
///
/// ## Arguments
///
/// * `args`: The parsed `RunArgs` struct containing all command-line options.
///
/// ## Returns
///
/// * `Result<()>`: Returns `Ok(())` on successful container start/run.
/// * `Err`: Returns an `Err` if configuration loading, path resolution, argument parsing,
///   or the underlying Docker operation fails (e.g., image pull fails, port conflict, container name conflict).
pub async fn handle_run(args: RunArgs) -> Result<()> {
    info!("Handling container run command..."); // Log entry point.
    debug!("Run args: {:?}", args); // Log parsed arguments.

    // 1. Load configuration (needed for potential default image prefix).
    let cfg = config::load_config().context("Failed to load DevRS configuration")?;

    // --- Determine Image Name ---
    // Use the user-provided image name or generate a default one.
    let image_name = match &args.image {
        Some(img) => img.clone(), // Use the explicitly provided image name.
        None => {
            // Generate default tag: <prefix>-<dirname>:latest
            let current_dir = env::current_dir().context("Failed to get current directory")?;
            let dir_name = current_dir
                .file_name()
                // Get the base name of the current directory.
                .map(|name| name.to_string_lossy().to_lowercase()) // Use lowercase name.
                .unwrap_or_else(|| "unknown-dir".to_string()); // Fallback if name extraction fails.

            // Get the configured prefix, if any.
            let prefix = cfg
                .application_defaults
                .default_image_prefix // Access config field.
                .as_deref() // Get Option<&str>.
                .filter(|p| !p.is_empty()) // Ignore empty prefixes.
                .map(|p| format!("{}-", p)) // Add hyphen if prefix exists.
                .unwrap_or_default(); // Use empty string otherwise.

            // Construct the default image tag.
            let default_image = format!("{}{}:latest", prefix, dir_name);
            info!(
                "No image specified via --image, using default based on directory: {}",
                default_image
            );
            default_image // Return the default tag.
        }
    };

    // 3. Check if the determined image exists locally. Issue a warning if not.
    // Docker will attempt to pull the image if it's not found locally when `run_container` is called.
    if !docker::image_exists(&image_name).await? {
        warn!(
            "Image '{}' not found locally. Docker will try to pull it.",
            image_name
        );
    }

    // --- Determine Container Name ---
    // Use the user-provided container name or generate a default one.
    let container_name = match &args.name {
        Some(name) => name.clone(), // Use the explicitly provided name.
        None => {
            // Generate default name: devrs-app-<dirname>
            let current_dir = env::current_dir().context("Failed to get current directory")?;
            let dir_name = current_dir
                .file_name()
                .map(|name| name.to_string_lossy().to_lowercase())
                .unwrap_or_else(|| "unknown".to_string()); // Fallback name part.
            let default_name = format!("devrs-app-{}", dir_name);
            info!(
                "No name specified via --name, using default: {}",
                default_name
            );
            default_name // Return the default name.
        }
    };

    // --- Prepare Environment Variables ---
    // Parse the `KEY=VALUE` strings provided via `--env` into a HashMap.
    let mut env_map: HashMap<String, String> = HashMap::new();
    for env_str in &args.env_vars {
        if let Some((key, value)) = env_str.split_once('=') {
            // Trim whitespace and insert into the map.
            env_map.insert(key.trim().to_string(), value.trim().to_string());
        } else {
            // Warn about invalid format but continue processing others.
            warn!(
                "Ignoring invalid environment variable format: '{}'. Expected KEY=VALUE.",
                env_str
            );
        }
    }

    // --- Prepare Mounts ---
    // Currently, `devrs container run` doesn't accept mount arguments directly.
    // Initialize an empty vector. Future enhancements could add CLI args for mounts.
    let mounts: Vec<config::MountConfig> = vec![]; // Pass empty vector for now.

    // --- Prepare Command Override ---
    // If the user provided trailing arguments after options, use them as the command.
    // Otherwise, pass `None` to use the image's default CMD/ENTRYPOINT.
    let command_override = if args.command.is_empty() {
        None
    } else {
        Some(args.command.clone()) // Clone the vector of command strings.
    };

    // --- Call Docker API Wrapper ---
    // Call the shared utility function to create and start the container.
    info!(
        "Attempting to run container '{}' from image '{}'",
        container_name, image_name
    );
    docker::run_container(
        &image_name, // Image to use.
        &container_name, // Name for the new container.
        &args.ports, // Port mappings (Vec<String>).
        &mounts, // Volume mounts (currently empty Vec<MountConfig>).
        &env_map, // Environment variables (HashMap).
        None, // workdir - use container's default (could be added as arg later).
        args.detach, // Run in background?
        args.rm, // Auto-remove on exit?
        command_override, // Optional command override.
    )
    .await // Await the async operation.
    .with_context(|| { // Add context to potential errors.
        format!(
            "Failed to run container '{}' from image '{}'",
            container_name, image_name
        )
    })?;

    // --- Report Success ---
    // Log success.
    info!(
        "Successfully started container '{}' from image '{}'",
        container_name, image_name
    );
    // Print different messages based on whether it was detached or ran in foreground.
    if args.detach {
        println!(
            "✅ Container '{}' started in detached mode.",
            container_name
        );
    } else {
        // If not detached, the `run_container` function likely waited for it to exit.
        // Note: Current `run_container` doesn't handle foreground I/O streaming well.
        println!(
            "✅ Container '{}' finished.", // Indicate the container process completed.
            container_name
        );
    }

    Ok(()) // Indicate overall success of the command.
}


// --- Unit Tests ---
// Focus on argument parsing. Testing `handle_run` logic requires mocking.
#[cfg(test)]
mod tests {
    use super::*;

    // Test parsing with a comprehensive set of arguments.
    #[test]
    fn test_run_args_parsing() {
        // Simulate `devrs container run --image myimage:v2 --name my-instance -p 80:8000 --port 443:8443 -e VAR1=val1 --env VAR2=val2 --detach --rm override_cmd --arg1`
        let args = RunArgs::try_parse_from(&[
            "run", // Command name context for clap.
            "--image",
            "myimage:v2",
            "--name",
            "my-instance",
            "-p", // Short port flag.
            "80:8000",
            "--port", // Long port flag.
            "443:8443",
            "-e", // Short env flag.
            "VAR1=val1",
            "--env", // Long env flag.
            "VAR2=val2",
            "--detach", // Detach flag.
            "--rm", // Auto-remove flag.
            "--", // Add separator before trailing command args
            "override_cmd", // Start of the command override.
            "--arg1", // Argument for the override command.
        ])
        .unwrap(); // Expect parsing to succeed.

        // Verify parsed values.
        assert_eq!(args.image, Some("myimage:v2".to_string()));
        assert_eq!(args.name, Some("my-instance".to_string()));
        assert_eq!(args.ports, vec!["80:8000", "443:8443"]);
        assert_eq!(args.env_vars, vec!["VAR1=val1", "VAR2=val2"]);
        assert!(args.detach);
        assert!(args.rm);
        assert_eq!(args.command, vec!["override_cmd", "--arg1"]);
    }

    // Test parsing with minimal arguments (only the required --image).
    #[test]
    fn test_run_args_parsing_minimal() {
        // Simulate `devrs container run --image minimal:tag`
        // Note: In the current implementation, --image is optional if a default can be generated.
        // This test assumes --image is provided for simplicity. If --image were truly required,
        // `try_parse_from(&["run"])` would fail. Let's test with image provided.
        let args = RunArgs::try_parse_from(&["run", "--image", "minimal:tag"]).unwrap();
        // Verify explicitly provided image is parsed.
        assert_eq!(args.image, Some("minimal:tag".to_string()));
        // Verify other optional fields are None or empty/false by default.
        assert!(args.name.is_none());
        assert!(args.ports.is_empty());
        assert!(args.env_vars.is_empty());
        assert!(!args.detach);
        assert!(!args.rm);
        assert!(args.command.is_empty()); // No command override.
    }

    // Test the handler logic conceptually (requires mocking).
    #[tokio::test]
    #[ignore] // Needs mocking `config::load_config`, `docker::image_exists`, `docker::run_container`.
    async fn test_handle_run_logic() {
        // --- Mocking Setup (Conceptual) ---
        // Mock `config::load_config` -> Ok(default_config)
        // Mock `docker::image_exists` -> Ok(true)
        // Mock `docker::run_container` -> Ok(()) and verify arguments passed to it.

        // Define arguments for the test.
        let args = RunArgs {
            image: Some("test:run".to_string()), // Explicit image.
            name: None, // Let name default.
            ports: vec!["8080:80".to_string()],
            env_vars: vec!["MODE=test".to_string()],
            detach: true,
            rm: true,
            command: vec![], // No command override.
        };

        // Execute the handler.
        let result = handle_run(args).await;
        // Expect success.
        assert!(result.is_ok());
        // Verify mocks: check `run_container` was called with expected derived container name,
        // image name "test:run", ports ["8080:80"], env {"MODE": "test"}, detach=true, rm=true, command=None.
    }
}
