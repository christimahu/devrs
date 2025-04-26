//! # DevRS Core Docker Operations
//!
//! File: cli/src/common/docker/operations.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!

//! ## Overview
//!
//! This module implements the fundamental, high-level Docker operations required
//! by various DevRS commands: building Docker images (`build_image`) and running
//! new containers (`run_container`). It acts as a primary interface to the `bollard`
//! crate for these core actions, handling configuration mapping, API calls, and
//! output streaming where applicable.
//!
//! ## Architecture
//!
//! Key functions provided:
//! - **`build_image`**:
//!   - Takes image tag, Dockerfile path, context directory, and cache options.
//!   - Creates a gzipped TAR archive of the build context directory using `common::archive::tar`.
//!   - Calls the Docker `build_image` API via `bollard`.
//!   - Streams the raw build output from Docker directly to standard output.
//!   - Handles build errors reported by Docker.
//! - **`run_container`**:
//!   - Takes image name, desired container name, port mappings, volume mount configurations (`config::MountConfig`), environment variables, working directory, detach flag, auto-remove flag, and an optional command override.
//!   - Converts DevRS `MountConfig` structs into the format required by `bollard` using `convert_mounts_to_bollard`.
//!   - Constructs the necessary `HostConfig` and `ContainerConfig` structures for the `bollard` API.
//!   - Checks if a container with the target name already exists using `state::container_exists` to prevent conflicts.
//!   - Calls the Docker `create_container` and `start_container` APIs via `bollard`.
//!   - Focuses *only* on creating and starting; does not handle waiting or log streaming for foreground processes (this is handled by `interaction::exec_in_container` or `interaction::get_container_logs`).
//!
//! Both functions utilize the shared `connect::connect_docker` helper.
//!
//! ## Usage
//!
//! These functions form the backbone for commands like `devrs env build`, `devrs container build`,
//! `devrs container run`, and are used internally by `lifecycle::ensure_core_env_running`.
//!
//! ```rust
//! use crate::common::docker::operations;
//! use crate::core::{config, error::Result};
//! use std::{collections::HashMap, path::Path};
//!
//! # async fn run_example() -> Result<()> {
//! // Example: Building an image
//! let tag = "my-app:latest";
//! let dockerfile = "Dockerfile";
//! let context_dir = ".";
//! operations::build_image(tag, dockerfile, context_dir, false).await?;
//!
//! // Example: Running a container
//! let image = "my-app:latest";
//! let container_name = "my-app-1";
//! let ports = vec!["8080:80".to_string()];
//! let mounts = vec![config::MountConfig { host: "/path/on/host".into(), container: "/data".into(), readonly: false }]; // Example mount
//! let env_vars = HashMap::from([("MODE".to_string(), "production".to_string())]);
//! operations::run_container(
//!     image,
//!     container_name,
//!     &ports,
//!     &mounts,
//!     &env_vars,
//!     Some("/app"), // workdir
//!     true,         // detach
//!     false,        // auto_remove
//!     None          // command (use image default)
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
use crate::common::archive::tar::create_context_tar; // Utility for creating build context tarball
use crate::core::config; // Use config structs (e.g., MountConfig)
use crate::core::error::{DevrsError, Result}; // Use standard Result and custom Error
use anyhow::{anyhow, Context}; // For error context wrapping
use bollard::{
    container::{
        // Structs needed for container configuration and creation
        Config as ContainerConfig,
        CreateContainerOptions,
        StartContainerOptions,
    },
    image::BuildImageOptions, // Options struct for image building
    models::{BuildInfo, HostConfig, Mount, MountTypeEnum, PortBinding}, // Data models from Docker API
                                                                        // Docker client is obtained via connect_docker
};
use futures_util::stream::StreamExt; // Required for processing streams (like build output)
use std::collections::HashMap; // For port bindings and env vars maps
use std::default::Default; // For default struct initialization
use std::io::{stdout, Write as IoWrite}; // For writing build output to stdout
use std::path::Path; // Filesystem path operations
use tracing::{debug, error, info, warn}; // Logging utilities

// Import necessary functions from sibling modules.
use super::connect::connect_docker; // Get Docker client connection
use super::state::container_exists; // Check for existing container before creating

// --- Image Building ---

/// Builds a Docker image using a specified Dockerfile and build context directory.
///
/// This function orchestrates the image build process:
/// 1. Creates a gzipped TAR archive of the `context_dir`.
/// 2. Connects to the Docker daemon.
/// 3. Calls the Docker `build_image` API via `bollard`.
/// 4. Streams the build output (stdout/stderr from Docker) directly to the host's standard output in real-time.
///
/// # Arguments
///
/// * `tag` - The desired name and tag for the image (e.g., "my-app:latest").
/// * `dockerfile` - The path to the Dockerfile, *relative to the root of the `context_dir`*.
/// * `context_dir` - The path to the directory containing the build context (files to be sent to Docker).
/// * `no_cache` - If `true`, instructs Docker to build without using its layer cache.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` on successful completion of the build stream.
///
/// # Errors
///
/// Returns an `Err` if:
/// - Creating the build context TAR archive fails.
/// - Connecting to the Docker daemon fails.
/// - The Docker `build_image` API call fails.
/// - An error occurs while streaming the build output.
/// - The Docker daemon reports a build error within the stream.
pub async fn build_image(
    tag: &str,
    dockerfile: &str,
    context_dir: &str,
    no_cache: bool,
) -> Result<()> {
    // Establish connection to Docker daemon.
    let docker = connect_docker().await?;
    let context_path = Path::new(context_dir);

    // Create the build context archive (gzipped tarball of the context directory).
    info!(
        "Creating build context tarball for '{}'...",
        context_path.display()
    );
    let tar_gz =
        create_context_tar(context_path).context("Failed to create build context tarball")?;
    info!("Build context created successfully.");

    // Configure options for the Docker build API call.
    let build_options = BuildImageOptions {
        dockerfile: dockerfile.to_string(), // Path to Dockerfile within the context.
        t: tag.to_string(),                 // Image name and tag.
        rm: true,                           // Remove intermediate containers after build.
        nocache: no_cache,                  // Use Docker build cache?
        // Add other options like buildargs, labels, target stage here if needed later.
        ..Default::default()
    };

    // Start the image build process.
    info!("Starting image build for tag: {}", tag);
    // `build_image` returns a stream of build events.
    let mut build_stream = docker.build_image(
        build_options,       // Pass the configured options.
        None,                // No custom registry auth config needed for local build.
        Some(tar_gz.into()), // Provide the build context as request body.
    );

    // Process the stream of build events from Docker.
    while let Some(build_result) = build_stream.next().await {
        match build_result {
            // Successfully received a build event.
            Ok(info) => match info {
                // Event contains standard output stream data.
                BuildInfo {
                    stream: Some(s), ..
                } => {
                    // Print the stream output directly to host stdout.
                    print!("{}", s);
                }
                // Event indicates a build error occurred.
                BuildInfo {
                    error: Some(err), // Error message from Docker.
                    error_detail,     // Optional detailed error information.
                    ..
                } => {
                    // Format the error message.
                    let detail_msg = error_detail.and_then(|d| d.message).unwrap_or_default();
                    error!("Build Error: {} - {}", err, detail_msg); // Log the detailed error.
                                                                     // Return our specific Docker build error.
                    return Err(anyhow!(DevrsError::Docker(format!(
                        "Build failed: {}. {}",
                        err, detail_msg
                    ))));
                }
                // Handle other status/progress messages (optional, log as debug).
                BuildInfo {
                    status: Some(s),
                    progress: Some(p),
                    ..
                } => {
                    debug!("Build Status: {}, Progress: {}", s, p);
                }
                BuildInfo {
                    status: Some(s), ..
                } => {
                    debug!("Build Status: {}", s);
                }
                // Log any other unhandled build info types.
                _ => debug!("Received unhandled build info: {:?}", info),
            },
            // An error occurred while receiving data from the build stream itself.
            Err(e) => {
                return Err(anyhow!(DevrsError::DockerApi { source: e }))
                    .context("Failed to process build stream event");
            }
        }
        // Ensure stdout is flushed frequently to show build progress in real-time.
        let _ = stdout().flush();
    }

    // If the loop completes without returning an error, the build stream finished successfully.
    info!("Image build stream finished successfully for tag: {}", tag);
    Ok(())
}

// --- Container Running ---

/// Creates and starts a new Docker container based on the provided configuration.
///
/// This function handles the two main steps: creating the container configuration
/// using the Docker API, and then starting the created container. It does *not*
/// wait for the container to exit if run in the foreground (`detach=false`) and
/// does not handle interactive I/O streaming (use `interaction::exec_in_container` for that).
///
/// # Arguments
///
/// * `image` - Name and tag of the Docker image to use (e.g., "my-app:latest").
/// * `name` - The desired name for the new container. Must be unique.
/// * `ports` - A slice of strings defining port mappings in "HOST:CONTAINER" format (e.g., `&["8080:80"]`).
/// * `mounts` - A slice of `config::MountConfig` structs defining volume mounts. Host paths must be absolute.
/// * `env_vars` - A `HashMap` containing environment variables (KEY=VALUE) to set inside the container.
/// * `workdir` - An optional path string for the working directory inside the container. If `None`, uses the image's default.
/// * `detach` - If `true`, the container runs in the background. If `false`, the container runs in the foreground (but this function doesn't wait or stream I/O). Also affects whether standard streams are attached by default.
/// * `auto_remove` - If `true`, Docker will automatically remove the container's filesystem when it exits. Useful for temporary tasks.
/// * `command` - An optional `Vec<String>` specifying a command and arguments to run, overriding the image's default `CMD` or `ENTRYPOINT`.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if the container is successfully created and started.
///
/// # Errors
///
/// Returns an `Err` if:
/// - Connecting to the Docker daemon fails (`DevrsError::DockerApi`).
/// - Preparing mounts fails due to invalid configuration (`DevrsError::Config`).
/// - A container with the specified `name` already exists (`DevrsError::DockerOperation`).
/// - Container creation or starting fails via the Docker API (`DevrsError::DockerApi`, potentially 404 if image not found during create).
#[allow(clippy::too_many_arguments)] // Necessary due to numerous container options
pub async fn run_container(
    image: &str,
    name: &str,
    ports: &[String],
    mounts: &[config::MountConfig],
    env_vars: &HashMap<String, String>,
    workdir: Option<&str>,
    detach: bool,
    auto_remove: bool,
    command: Option<Vec<String>>,
) -> Result<()> {
    // Establish connection to Docker daemon.
    let docker = connect_docker().await?;

    // --- Prepare HostConfig (Networking, Mounts, Resources) ---
    // Parse port mapping strings into the structure required by bollard.
    let mut port_bindings: HashMap<String, Option<Vec<PortBinding>>> = HashMap::new();
    // Keep track of ports that need to be formally exposed by the container config.
    let mut exposed_ports: HashMap<String, HashMap<(), ()>> = HashMap::new(); // Value is just {}

    for mapping in ports {
        // Expect "HOST:CONTAINER" or "HOST:CONTAINER/proto" format.
        if let Some((host_part, container_part)) = mapping.split_once(':') {
            // Check for protocol specification (e.g., "80/tcp"). Default to tcp.
            let (container_port, proto) =
                if let Some((port, protocol)) = container_part.split_once('/') {
                    (port, format!("/{}", protocol.to_lowercase())) // Ensure proto includes '/' and is lowercase
                } else {
                    (container_part, "/tcp".to_string()) // Default to TCP
                };
            // Key format expected by Docker API: "port/proto" (e.g., "80/tcp").
            let container_port_proto = format!("{}{}", container_port, proto);

            // Mark the port as exposed in the container config.
            exposed_ports.insert(container_port_proto.clone(), HashMap::new()); // Value is empty map.

            // Create the host port binding specification.
            let binding = PortBinding {
                host_ip: None, // Bind to all host interfaces by default. Could be specified if needed.
                host_port: Some(host_part.to_string()), // The host port to map to.
            };
            // Add the binding to the map, handling potential multiple bindings for one container port (rare).
            port_bindings
                .entry(container_port_proto)
                .or_default() // Get Option<Vec>, default to None then insert Some(Vec::new())
                .get_or_insert_with(Vec::new) // Ensure Vec exists
                .push(binding);
        } else {
            // Log invalid port mapping formats.
            warn!("Ignoring invalid port mapping format: {}", mapping);
        }
    }

    // Convert the application's MountConfig structs into bollard's Mount structs.
    // This helper also validates paths.
    let bollard_mounts =
        convert_mounts_to_bollard(mounts).context("Failed to prepare container mounts")?;

    // Construct the HostConfig part of the container creation request.
    let host_config = HostConfig {
        // Add port bindings if any were defined.
        port_bindings: if port_bindings.is_empty() {
            None
        } else {
            Some(port_bindings)
        },
        // Set the auto-remove flag based on the argument.
        auto_remove: Some(auto_remove),
        // Add volume mounts if any were defined.
        mounts: if bollard_mounts.is_empty() {
            None
        } else {
            Some(bollard_mounts)
        },
        // Add other host config options here if needed (e.g., resource limits, network mode).
        ..Default::default()
    };

    // --- Prepare ContainerConfig (Image, Command, Env, Standard Streams) ---
    // Format environment variables into the "KEY=VALUE" string list required by Docker API.
    let env_list: Vec<String> = env_vars
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();

    // Determine if standard streams should be attached. Generally true for foreground, false for detached.
    let attach_streams = !detach;

    // Construct the main ContainerConfig part of the request.
    let config = ContainerConfig {
        image: Some(image.to_string()), // Image name and tag.
        // Add environment variables if any were provided.
        env: if env_list.is_empty() {
            None
        } else {
            Some(env_list)
        },
        // Set the command override if provided.
        cmd: command,
        // Set the working directory if provided.
        working_dir: workdir.map(String::from),
        // Add exposed ports declaration if any ports were mapped.
        exposed_ports: if exposed_ports.is_empty() {
            None
        } else {
            Some(exposed_ports)
        },
        // Attach the HostConfig created above.
        host_config: Some(host_config),
        // Configure standard stream attachment based on 'detach' flag.
        attach_stdout: Some(attach_streams),
        attach_stderr: Some(attach_streams),
        attach_stdin: Some(attach_streams), // Attach stdin if running foreground
        open_stdin: Some(attach_streams),   // Keep stdin open if running foreground
        // Allocate TTY usually matches stream attachment for basic interaction.
        // For fine-grained control (e.g., non-interactive foreground), callers might use
        // interaction::exec_in_container or lower-level API calls.
        tty: Some(attach_streams),
        // Add other container config options here if needed (e.g., labels, user).
        ..Default::default()
    };

    // --- Check for Existing Container with Same Name ---
    // Prevent Docker error by checking if a container with the target name already exists.
    if container_exists(name).await? {
        error!(
            "Container named '{}' already exists. Use a different name or remove the existing container first.",
            name
        );
        // Return specific error indicating the name conflict.
        return Err(anyhow!(DevrsError::DockerOperation(format!(
            "Container named '{}' already exists. Cannot create.",
            name
        ))));
    }

    // --- Create and Start the Container ---
    info!("Creating container '{}' from image '{}'", name, image); // Log action.
                                                                   // Define options for the create_container API call (primarily the name).
    let create_options = Some(CreateContainerOptions {
        name: name.to_string(),
        platform: None, // Platform override (optional).
    });
    // Make the API call to create the container.
    let container_info = docker
        .create_container(create_options, config)
        .await
        // Map potential errors (like image not found 404).
        .map_err(|e| anyhow!(DevrsError::DockerApi { source: e }))
        .with_context(|| format!("Failed to create container '{}'", name))?;

    // Container created successfully, now start it.
    info!("Starting container '{}' (ID: {})", name, container_info.id); // Log start action.
                                                                        // Make the API call to start the container.
    docker
        .start_container(name, None::<StartContainerOptions<String>>) // No specific start options needed
        .await
        // Map potential errors during start.
        .map_err(|e| anyhow!(DevrsError::DockerApi { source: e }))
        .with_context(|| format!("Failed to start container '{}'", name))?;

    // Container started successfully.
    info!("Container '{}' started successfully.", name);

    // Note: This function finishes here. It does *not* wait for foreground containers
    // to exit or stream their logs. That responsibility lies with the caller or
    // should be handled using functions from the `interaction` module if needed.

    Ok(()) // Indicate overall success of creation and start.
}

/// Converts DevRS `MountConfig` structures into `bollard::models::Mount` structures.
///
/// This helper function translates the mount configuration defined in the application's
/// config format into the specific format required by the Docker API via `bollard`.
/// It also performs validation to ensure host paths are absolute and container paths
/// are valid absolute paths.
///
/// # Arguments
///
/// * `mounts_config` - A slice of `config::MountConfig` structs from the application's configuration.
///
/// # Returns
///
/// * `Result<Vec<Mount>>` - A vector of `bollard::models::Mount` structs ready for use in `HostConfig`.
///
/// # Errors
///
/// Returns `DevrsError::Config` if:
/// - Any `host` path in `mounts_config` is not absolute (after initial config path expansion).
/// - Any `container` path is empty or not absolute (does not start with '/').
fn convert_mounts_to_bollard(mounts_config: &[config::MountConfig]) -> Result<Vec<Mount>> {
    // Vector to store the converted bollard Mount structs.
    let mut bollard_mounts = Vec::new();
    // Iterate through the application's mount configurations.
    for mc in mounts_config {
        let host_path = Path::new(&mc.host);
        // Validate host path (should already be absolute after config load).
        if !host_path.is_absolute() {
            // Log warning, as this indicates an issue earlier in config processing.
            warn!(
                "Non-absolute host path found during mount conversion: '{}'. This might indicate an issue.",
                mc.host
            );
            return Err(anyhow!(DevrsError::Config(format!(
                "Host path '{}' for mount must be absolute. Check config loading/expansion.",
                mc.host
            ))));
        }
        // Validate container path (must be absolute and non-empty).
        if mc.container.is_empty() || !mc.container.starts_with('/') {
            return Err(anyhow!(DevrsError::Config(format!(
                "Container path '{}' for mount must be absolute and non-empty.",
                mc.container
            ))));
        }

        // Create the bollard Mount struct.
        bollard_mounts.push(Mount {
            target: Some(mc.container.clone()), // Path inside the container.
            source: Some(mc.host.clone()),      // Path on the host system.
            typ: Some(MountTypeEnum::BIND), // Specify mount type as 'bind'. Volume mounts would use MountTypeEnum::VOLUME.
            read_only: Some(mc.readonly),   // Apply read-only flag.
            consistency: None,              // Default consistency.
            bind_options: None,             // No specific bind options needed.
            volume_options: None,           // Not a volume mount.
            tmpfs_options: None,            // Not a tmpfs mount.
        });
    }
    // Return the vector of converted mount configurations.
    Ok(bollard_mounts)
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*; // Import items from parent module.
    use crate::core::config::MountConfig; // Import needed config struct.

    // Test successful conversion of valid MountConfig entries.
    #[test]
    fn test_convert_mounts_to_bollard_valid() {
        // Define sample valid mount configurations.
        let configs: Vec<MountConfig> = vec![
            MountConfig {
                host: "/home/user/code".into(), // Absolute host path
                container: "/code".into(),      // Absolute container path
                readonly: false,
            },
            MountConfig {
                host: "/etc/config.toml".into(),      // Absolute host path
                container: "/app/config.toml".into(), // Absolute container path
                readonly: true,
            },
        ];
        // Perform the conversion.
        let result = convert_mounts_to_bollard(&configs);
        // Expect success.
        assert!(result.is_ok());
        let bollard_mounts = result.unwrap();
        // Verify the number of resulting mounts.
        assert_eq!(bollard_mounts.len(), 2);

        // Verify details of the first mount.
        assert_eq!(bollard_mounts[0].source.as_deref(), Some("/home/user/code"));
        assert_eq!(bollard_mounts[0].target.as_deref(), Some("/code"));
        assert_eq!(bollard_mounts[0].read_only, Some(false));
        assert_eq!(bollard_mounts[0].typ, Some(MountTypeEnum::BIND));

        // Verify details of the second mount.
        assert_eq!(
            bollard_mounts[1].source.as_deref(),
            Some("/etc/config.toml")
        );
        assert_eq!(
            bollard_mounts[1].target.as_deref(),
            Some("/app/config.toml")
        );
        assert_eq!(bollard_mounts[1].read_only, Some(true));
        assert_eq!(bollard_mounts[1].typ, Some(MountTypeEnum::BIND));
    }

    // Test conversion failure with a non-absolute host path.
    // Note: This should ideally be caught during config loading/expansion,
    // but the validation here acts as a safeguard.
    #[test]
    fn test_convert_mounts_to_bollard_non_absolute_host() {
        let configs = vec![MountConfig {
            host: "relative/path".into(), // Invalid host path
            container: "/code".into(),
            readonly: false,
        }];
        let result = convert_mounts_to_bollard(&configs);
        // Expect an error.
        assert!(result.is_err());
        // Check the error message content.
        assert!(result.unwrap_err().to_string().contains("must be absolute"));
    }

    // Test conversion failure with invalid container paths (relative or empty).
    #[test]
    fn test_convert_mounts_to_bollard_invalid_container_path() {
        // Case 1: Relative container path.
        let configs_relative = vec![MountConfig {
            host: "/absolute/host".into(),
            container: "relative".into(), // Invalid container path
            readonly: false,
        }];
        let result_relative = convert_mounts_to_bollard(&configs_relative);
        assert!(result_relative.is_err());
        assert!(result_relative
            .unwrap_err()
            .to_string()
            // Check specific error message content.
            .contains("Container path 'relative' for mount must be absolute and non-empty."));

        // Case 2: Empty container path.
        let configs_empty = vec![MountConfig {
            host: "/absolute/host".into(),
            container: "".into(), // Invalid empty container path
            readonly: false,
        }];
        let result_empty = convert_mounts_to_bollard(&configs_empty);
        assert!(result_empty.is_err());
        assert!(result_empty
            .unwrap_err()
            .to_string()
            // Check specific error message content.
            .contains("Container path '' for mount must be absolute and non-empty."));
    }

    // Placeholder test to ensure the module compiles.
    #[test]
    fn placeholder_operations_test() {
        assert!(true);
    }

    // TODO: Add mocked tests for `build_image` and `run_container`.
    // - build_image: Verify context creation, bollard call args, stream handling.
    // - run_container: Verify config mapping (ports, mounts, env), bollard create/start calls, error handling (conflict, image not found).
}
