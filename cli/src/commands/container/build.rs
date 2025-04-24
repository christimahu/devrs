//! # DevRS Container Build Command
//!
//! File: cli/src/commands/container/build.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs container build` command, which builds
//! Docker images for application containers, typically based on a `Dockerfile`
//! found in the current working directory or a specified location.
//!
//! ## Architecture
//!
//! The command flow follows these steps:
//! 1. Parse command arguments (tag, Dockerfile path, cache options).
//! 2. Load global DevRS configuration to check for a default image prefix.
//! 3. Determine the final image tag to use: either the one provided via `--tag`
//!    or generate a default tag based on the current directory name and the
//!    optional prefix from the configuration (`<prefix>-<dirname>:latest`).
//! 4. Validate the path to the Dockerfile to ensure it exists and is a file.
//! 5. Set the build context directory (currently always the current working directory).
//! 6. Invoke the shared Docker build utility (`common::docker::build_image`)
//!    with the determined tag, Dockerfile path (relative to context), context path,
//!    and cache options.
//! 7. Display build progress streamed from Docker and report final success or failure.
//!
//! ## Examples
//!
//! Usage examples:
//!
//! ```bash
//! # Build image using Dockerfile in current dir, tag based on dir name
//! devrs container build
//!
//! # Build with explicit tag
//! devrs container build --tag myapp:1.0
//!
//! # Build with custom Dockerfile name relative to current dir
//! devrs container build --file Dockerfile.prod --tag myapp:prod
//!
//! # Build without using Docker's cache
//! devrs container build --no-cache
//! ```
//!
//! The command provides feedback during the build process by streaming Docker's output.
//!
use crate::common::docker; // Access shared Docker utilities (like build_image).
use crate::core::config; // Access configuration loading.
use crate::core::error::Result; // Standard Result type for error handling.
use anyhow::Context; // For adding context to errors.
use clap::Parser; // For parsing command-line arguments.
use std::env; // For getting the current working directory.
use tracing::{debug, info}; // Logging framework utilities.

/// # Container Build Arguments (`BuildArgs`)
///
/// Defines the command-line arguments accepted by the `devrs container build` subcommand.
/// Uses the `clap` crate for parsing and validation.
#[derive(Parser, Debug)]
pub struct BuildArgs {
    /// Optional: Specifies the name and tag for the image being built (e.g., "my-app:1.0", "repo/image:latest").
    /// If omitted, a default tag is generated based on the current directory name and an optional
    /// prefix defined in the DevRS configuration (`application_defaults.default_image_prefix`).
    /// The default format is `<prefix>-<directory_name>:latest` or `<directory_name>:latest` if no prefix is set.
    #[arg(short, long)] // Define as `--tag` or `-t`
    tag: Option<String>,

    /// Optional: Specifies the path to the Dockerfile, relative to the current working directory (build context).
    /// Defaults to "Dockerfile" if not provided.
    #[arg(short, long, default_value = "Dockerfile")] // Define as `--file` or `-f`
    file: String,

    /// Optional: If set, instructs Docker to build the image without using its layer cache.
    /// This ensures all build steps are re-executed.
    #[arg(long)] // Define as `--no-cache`
    no_cache: bool,
    // TODO: Consider adding other common Docker build options like `--build-arg KEY=VALUE` if needed in the future.
}

/// # Handle Container Build Command (`handle_build`)
///
/// The main asynchronous handler function for the `devrs container build` command.
/// It takes the parsed arguments, determines the image tag and Dockerfile path,
/// and then invokes the underlying Docker build process via the `common::docker` module.
///
/// ## Workflow:
/// 1.  Logs the start and the parsed arguments.
/// 2.  Loads the global DevRS configuration to retrieve the optional default image prefix.
/// 3.  Determines the final image tag: uses `--tag` if provided, otherwise generates a default tag
///     (`<prefix>-<directory_name>:latest`) using the current directory's name and the loaded prefix.
/// 4.  Resolves the absolute path to the specified Dockerfile (relative to the current directory)
///     and validates its existence and that it's a file.
/// 5.  Sets the build context path (always the current working directory, specified as ".").
/// 6.  Calls the shared `docker::build_image` function, passing the final tag, the *relative* Dockerfile path
///     specified by the user (or the default "Dockerfile"), the context path ("."), and the `no_cache` flag.
/// 7.  Streams build output from Docker to the console.
/// 8.  Prints a final success message or propagates an error if the build fails.
///
/// ## Arguments
///
/// * `args`: The parsed `BuildArgs` struct containing command-line options.
///
/// ## Returns
///
/// * `Result<()>`: Returns `Ok(())` on successful image build, or an `Err` if configuration loading,
///   path validation, or the Docker build process fails.
pub async fn handle_build(args: BuildArgs) -> Result<()> {
    info!("Handling container build command..."); // Log entry point.
    debug!("Build args: {:?}", args); // Log parsed arguments for debugging.

    // 1. Load configuration to potentially get a default image prefix.
    let cfg = config::load_config().context("Failed to load DevRS configuration")?;

    // 2. Determine the final image tag.
    let image_tag = match &args.tag {
        // If a tag was provided via `--tag`, use it directly.
        Some(tag) => tag.clone(),
        // If no tag was provided, generate a default one.
        None => {
            // Get the current working directory path.
            let current_dir = env::current_dir().context("Failed to get current directory")?;
            // Extract the directory name, convert to lowercase, or use a fallback.
            let dir_name = current_dir
                .file_name()
                .map(|name| name.to_string_lossy().to_lowercase()) // Use lowercase dir name.
                .unwrap_or_else(|| "unknown-dir".to_string()); // Fallback name.

            // Get the optional prefix from the loaded config.
            let prefix = cfg
                .application_defaults
                .default_image_prefix // Access the prefix field.
                .as_deref() // Convert Option<String> to Option<&str>.
                .filter(|p| !p.is_empty()) // Use prefix only if it's not an empty string.
                .map(|p| format!("{}-", p)) // Add a hyphen if prefix exists.
                .unwrap_or_default(); // Use empty string if no prefix or empty prefix.

            // Construct the default tag: <prefix>-<dirname>:latest
            let default_tag = format!("{}{}:latest", prefix, dir_name);
            info!("No tag specified, using default: {}", default_tag); // Log the generated tag.
            default_tag // Return the generated default tag.
        }
    };

    // 3. Determine and validate the path to the Dockerfile.
    // Get the current working directory, which acts as the build context root.
    let current_dir = env::current_dir().context("Failed to get current directory")?;
    // Join the CWD with the filename provided by the `--file` arg (defaults to "Dockerfile").
    let dockerfile_path = current_dir.join(&args.file);

    // Check if the resolved Dockerfile path exists.
    if !dockerfile_path.exists() {
        anyhow::bail!( // Return an error if not found.
            "Dockerfile not found at expected path: {}",
            dockerfile_path.display()
        );
    }
    // Check if the path points to a regular file.
    if !dockerfile_path.is_file() {
        anyhow::bail!( // Return an error if it's not a file (e.g., it's a directory).
            "Specified Dockerfile path is not a file: {}",
            dockerfile_path.display()
        );
    }
    // IMPORTANT: We pass the *original* (potentially relative) path from `args.file`
    // to the `docker::build_image` function below, because Docker interprets
    // the Dockerfile path relative to the build context root.
    let dockerfile_arg = &args.file;
    info!("Using Dockerfile: {}", dockerfile_path.display()); // Log the absolute path for user info.

    // 4. Set the build context directory path. For `devrs container build`,
    // this is always the current working directory. Represented as "." for Docker.
    let context_dir = ".";
    info!("Using build context: {}", current_dir.display()); // Log absolute path of context.

    // 5. Initiate the Docker image build process.
    info!("Starting Docker build for image '{}'...", image_tag);
    // Call the shared build function from the common::docker module.
    docker::build_image(
        &image_tag,       // The final tag for the image.
        dockerfile_arg,   // The relative path to the Dockerfile within the context.
        context_dir,      // The build context path (".").
        args.no_cache,    // The boolean flag for using Docker cache.
    )
    .await // Await the async build process.
    .with_context(|| format!("Failed to build Docker image '{}'", image_tag))?; // Add context on error.

    // 6. Log and print success message.
    info!("Successfully built Docker image '{}'", image_tag);
    println!("✅ Successfully built image: {}", image_tag);

    Ok(()) // Indicate overall success.
}


// --- Unit Tests ---
// Tests focus on argument parsing and the logic for determining the image tag.
// Testing the actual call to `docker::build_image` requires mocking the Docker API.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{ApplicationDefaults, Config}; // Import necessary config structs for mocking.
    use std::{env, fs};
    use tempfile::tempdir; // For creating temporary directories during tests.

    // Helper function to create a mock Config object with an optional image prefix.
    // Used to simulate different configuration scenarios in tests.
    #[allow(dead_code)] // Allow this helper to be unused if tests change.
    fn mock_config(prefix: Option<&str>) -> Config {
        Config {
            // Set the application defaults section with the provided prefix.
            application_defaults: ApplicationDefaults {
                default_image_prefix: prefix.map(String::from), // Convert &str to Option<String>.
                ..Default::default() // Use default values for other fields.
            },
            // Use default values for other top-level config sections.
            ..Default::default()
        }
    }

    // Helper function to set up a temporary test environment.
    // Creates a temporary directory, adds a dummy Dockerfile inside it,
    // and changes the current working directory to the temp directory for the test's scope.
    // Returns the TempDir guard to ensure the directory is cleaned up afterwards.
    fn setup_test_env() -> tempfile::TempDir {
        let temp_dir = tempdir().unwrap(); // Create temp dir.
        // Create a minimal valid Dockerfile.
        fs::write(temp_dir.path().join("Dockerfile"), "FROM scratch").unwrap();
        // Change CWD to the temp directory. This is crucial because the default tag
        // generation and Dockerfile path resolution depend on the CWD.
        // Note: Changing CWD can affect parallel tests; consider using libraries
        // that isolate CWD if running tests in parallel becomes an issue.
        env::set_current_dir(temp_dir.path()).unwrap();
        temp_dir // Return the guard, which cleans up the dir when dropped.
    }

    // Test case: User provides an explicit tag via --tag.
    #[tokio::test]
    #[ignore] // Requires mocking `config::load_config` and `docker::build_image`
    async fn test_handle_build_with_tag() {
        let _temp_dir = setup_test_env(); // Setup env, ensure Dockerfile exists.

        // --- Mocking (Conceptual) ---
        // Mock `config::load_config` to return a default config (prefix doesn't matter here).
        // Mock `docker::build_image` to expect a call with the specific tag "my-app:v1".

        // Define arguments with an explicit tag.
        let args = BuildArgs {
            tag: Some("my-app:v1".to_string()),
            file: "Dockerfile".to_string(), // Use default Dockerfile name.
            no_cache: false,
        };

        // Execute the handler.
        let result = handle_build(args).await;
        // Expect success.
        assert!(result.is_ok());
        // Verify (via mock) that `docker::build_image` was called with tag "my-app:v1".
    }

    // Test case: User does not provide a tag, no prefix configured.
    #[tokio::test]
    #[ignore] // Requires mocking `config::load_config` and `docker::build_image`
    async fn test_handle_build_default_tag_no_prefix() {
        // --- Mocking (Conceptual) ---
        // Mock `config::load_config` to return a config *without* an application prefix.

        // Define arguments without an explicit tag.
        let args = BuildArgs {
            tag: None, // Let it default.
            file: "Dockerfile".to_string(),
            no_cache: true, // Test with no_cache flag too.
        };

        // Execute the handler.
        let result = handle_build(args).await;
        // Expect success.
        assert!(result.is_ok());
        // Verify (via mock) that `docker::build_image` was called with the correct default tag.
    }

    // Test case: User does not provide a tag, but a prefix *is* configured.
    #[tokio::test]
    #[ignore] // Requires mocking `config::load_config` and `docker::build_image`
    async fn test_handle_build_default_tag_with_prefix() {
        // Define arguments without an explicit tag.
        let args = BuildArgs {
            tag: None, // Let it default.
            file: "Dockerfile".to_string(),
            no_cache: false,
        };

        // Execute the handler.
        let result = handle_build(args).await;
        // Expect success.
        assert!(result.is_ok());
        // Verify (via mock) that `docker::build_image` was called with the correct default tag including prefix.
    }

    // Test case: The specified Dockerfile (or default) does not exist.
    #[tokio::test]
    #[ignore] // Requires mocking `config::load_config`
    async fn test_handle_build_missing_dockerfile() {
        let temp_dir = tempdir().unwrap(); // Create an *empty* temp dir.
        // Change CWD to the empty temp dir.
        env::set_current_dir(temp_dir.path()).unwrap();

        // --- Mocking (Conceptual) ---
        // Mock `config::load_config` to return a default config.

        // Define arguments using the default Dockerfile name, which doesn't exist here.
        let args = BuildArgs {
            tag: Some("test:fail".to_string()),
            file: "Dockerfile".to_string(), // Default, but does not exist.
            no_cache: false,
        };

        // Execute the handler.
        let result = handle_build(args).await;
        // Expect an error.
        assert!(result.is_err());
        // Verify the error message indicates the Dockerfile was not found.
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Dockerfile not found"));
    }

    // Test case: User specifies a custom Dockerfile name using --file.
    #[tokio::test]
    #[ignore] // Requires mocking `config::load_config` and `docker::build_image`
    async fn test_handle_build_custom_dockerfile() {
        let temp_dir = setup_test_env(); // Setup env (creates default "Dockerfile").
        // Create the *custom* Dockerfile that the user will specify.
        fs::write(
            temp_dir.path().join("Dockerfile.dev"), // Custom filename.
            "FROM scratch AS dev",
        )
        .unwrap();

        // --- Mocking (Conceptual) ---
        // Mock `config::load_config` to return a default config.
        // Mock `docker::build_image` to expect a call where the `dockerfile_arg` is "Dockerfile.dev".

        // Define arguments specifying the custom Dockerfile.
        let args = BuildArgs {
            tag: Some("test:custom".to_string()),
            file: "Dockerfile.dev".to_string(), // Point to the custom file.
            no_cache: false,
        };

        // Execute the handler.
        let result = handle_build(args).await;
        // Expect success.
        assert!(result.is_ok());
        // Verify (via mock) that `docker::build_image` was called with `dockerfile_arg` set to "Dockerfile.dev".
    }
}
