//! # DevRS Environment Build Command
//!
//! File: cli/src/commands/env/build.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs env build` command. Its primary responsibility
//! is to build (or rebuild) the **core development environment Docker image**.
//! This image, defined by `presets/Dockerfile.devrs` within the repository, contains
//! all the common tools and dependencies needed for development across various projects
//! managed with DevRS.
//!
//! ## Architecture
//!
//! The command flow follows these steps:
//! 1. Parse command-line arguments (`BuildArgs`) using `clap`, specifically the `--no-cache` and `--stage` flags.
//! 2. Attempt to load the DevRS configuration (`core::config`) to retrieve the intended `image_name` and `image_tag` for the core environment. Fallback to hardcoded defaults ("devrs-core-env", "latest") if configuration loading fails or specific values are missing. Note: The default image name was changed from "devrs-env" to "devrs-core-env" for clarity.
//! 3. Construct the full image tag (e.g., "my-custom-env:beta", "devrs-core-env:latest"). Validate that the resulting name and tag are not empty.
//! 4. Define the expected location of the Dockerfile (`presets/Dockerfile.devrs`) relative to the repository root and the build context (repository root, represented as ".").
//! 5. Validate that the `Dockerfile.devrs` exists at the expected path (`presets/Dockerfile.devrs`). **Assumption:** This command is run from the DevRS repository root directory.
//! 6. Call the shared Docker utility function `common::docker::build_image` with the final image tag, relative Dockerfile path, context path ("."), and cache options.
//! 7. Stream build output from Docker to the console.
//! 8. Report final success or failure.
//!
//! ## Examples
//!
//! Usage examples (run from the repository root):
//!
//! ```bash
//! # Build the core environment image using defaults and cache
//! devrs env build
//!
//! # Build without using Docker's layer cache
//! devrs env build --no-cache
//!
//! # Build only up to a specific stage (Note: Currently informational only)
//! devrs env build --stage base-tools
//! ```
//!
//! The command builds the single, shared core development environment image, not project-specific application images (which are handled by `devrs container build`).
//!
use crate::{
    common::docker, // Access shared Docker utilities (specifically build_image).
    core::{config, error::Result}, // Standard Result type for error handling & config loading.
};
use anyhow::{bail, Context}; // For adding context to errors & early returns.
use clap::Parser; // For parsing command-line arguments.
use std::path::Path; // For working with filesystem paths (Dockerfile location).
use tracing::{debug, info, warn}; // Logging framework utilities.

// Define the relative path to the core environment Dockerfile within the repository.
const CORE_DOCKERFILE_PATH: &str = "presets/Dockerfile.devrs";
// Define the default image name if config is missing or empty.
const DEFAULT_CORE_IMAGE_NAME: &str = "devrs-core-env";
// Define the default image tag if config is missing or empty.
const DEFAULT_CORE_IMAGE_TAG: &str = "latest";

/// # Environment Build Arguments (`BuildArgs`)
///
/// Defines the command-line arguments accepted by the `devrs env build` subcommand.
/// Uses the `clap` crate for parsing and validation.
#[derive(Parser, Debug)]
#[command(
    about = "Build the core DevRS development environment Docker image",
    long_about = "Builds the main Docker image containing the shared development toolchain.\n\
                  Uses the 'presets/Dockerfile.devrs' file within the repository."
)]
pub struct BuildArgs {
    /// Optional: If set, instructs Docker to build the image without using its layer cache.
    /// This forces all steps in the Dockerfile to be re-executed, which can be useful
    /// for ensuring updates or debugging build issues, but is generally slower.
    #[arg(long)] // Define as `--no-cache`.
    no_cache: bool,

    /// Optional: Specifies a target stage to build up to within a multi-stage Dockerfile.
    /// Example: `--stage base-tools`.
    /// **Note:** While the argument is parsed, the underlying build logic in `common::docker::build_image`
    /// does not currently pass the `--target` flag to Docker. This option is currently informational.
    #[arg(long)] // Define as `--stage <STAGE_NAME>`.
    stage: Option<String>,
}

/// # Handle Environment Build Command (`handle_build`)
///
/// The main asynchronous handler function for the `devrs env build` command.
/// It orchestrates the process of building the core development environment Docker image
/// based on `presets/Dockerfile.devrs` and configuration settings.
///
/// ## Workflow:
/// 1.  Logs the start and parsed arguments. Notes if `--stage` is used but not fully implemented.
/// 2.  Defines hardcoded default values for the image name and tag.
/// 3.  Attempts to load the DevRS configuration using `core::config::load_config`.
/// 4.  Determines the final image name and tag, using config values if present and non-empty, otherwise falling back to defaults.
/// 5.  Constructs the full image tag string (e.g., `image_name:image_tag`). Validates it's not empty.
/// 6.  Defines the relative path to the Dockerfile (`presets/Dockerfile.devrs`) and the build context directory (".").
/// 7.  Validates that the Dockerfile exists and is a file in the current working directory (assumed to be the repository root).
/// 8.  Calls the shared `common::docker::build_image` function, providing the full image tag, relative Dockerfile path,
///     context path ("."), and the `no_cache` flag.
/// 9.  Streams Docker build output to the console.
/// 10. Logs and prints a success message upon completion.
///
/// ## Arguments
///
/// * `args`: The parsed `BuildArgs` struct containing the command-line options (`no_cache`, `stage`).
///
/// ## Returns
///
/// * `Result<()>`: Returns `Ok(())` on successful image build.
/// * `Err`: Returns an `Err` if configuration loading fails critically,
///   if the Dockerfile is not found, or if the Docker build process itself fails.
pub async fn handle_build(args: BuildArgs) -> Result<()> {
    info!("Handling env build command..."); // Log entry point.
    debug!("Build args: {:?}", args); // Log parsed arguments.

    // Check if the --stage argument was used and warn that it's not fully implemented yet.
    if let Some(stage_name) = &args.stage {
        warn!(
            "The --stage={} argument is provided but not fully implemented in the build process yet.",
            stage_name
        );
        // Note: To implement this, the `common::docker::build_image` function would need
        // to be updated to accept and pass a 'target' option to bollard's BuildImageOptions.
    }

    // --- Determine Image Name and Tag ---
    // Initialize with defaults.
    let mut image_name = DEFAULT_CORE_IMAGE_NAME.to_string();
    let mut image_tag = DEFAULT_CORE_IMAGE_TAG.to_string();

    // Attempt to load configuration.
    match config::load_config() {
        Ok(cfg) => {
            info!("Successfully loaded configuration.");
            // Use configured name if not empty, else keep default.
            if !cfg.core_env.image_name.trim().is_empty() {
                image_name = cfg.core_env.image_name;
            } else {
                info!(
                    "Config 'core_env.image_name' is empty, using default '{}'",
                    DEFAULT_CORE_IMAGE_NAME
                );
            }
            // Use configured tag if not empty, else keep default.
            if !cfg.core_env.image_tag.trim().is_empty() {
                image_tag = cfg.core_env.image_tag;
            } else {
                info!(
                    "Config 'core_env.image_tag' is empty, using default '{}'",
                    DEFAULT_CORE_IMAGE_TAG
                );
            }
        }
        Err(e) => {
            // Config loading failed. Log warning and proceed with defaults.
            warn!(
                "Could not load configuration ({}). Proceeding with default image name '{}' and tag '{}'.",
                e, DEFAULT_CORE_IMAGE_NAME, DEFAULT_CORE_IMAGE_TAG
            );
        }
    }

    // Construct the full image tag string (e.g., "image_name:image_tag").
    let full_image_tag = format!("{}:{}", image_name, image_tag);
    info!("Using image target: {}", full_image_tag); // Log the final tag being used.

    // --- Sanity Check ---
    // This check should ideally not fail given the default fallbacks, but added for safety.
    if image_name.trim().is_empty() || image_tag.trim().is_empty() {
        bail!( // Use anyhow::bail! for a direct error return.
            "Internal error: image name ('{}') or tag ('{}') is empty after config processing. Cannot build image.",
            image_name, image_tag
        );
    }

    // --- Define Build Paths ---
    // Define the relative path to the Dockerfile.
    let dockerfile_path_str = CORE_DOCKERFILE_PATH;
    // Define the build context directory (always the current directory, represented as ".").
    let context_dir_str = ".";
    // Create Path objects for validation.
    let dockerfile_path = Path::new(dockerfile_path_str);
    let context_dir = Path::new(context_dir_str); // Represents current directory.

    // --- Validate Dockerfile Existence ---
    // Check if the Dockerfile exists and is a file in the expected location relative to CWD.
    // Assumes the command is run from the repository root.
    if !dockerfile_path.is_file() { // Use is_file() for better check
        bail!(
            "Core environment Dockerfile not found at expected path: '{}'. Please run this command from the root of the devrs repository.",
            dockerfile_path.display()
        );
    }
    info!("Using Dockerfile: {}", dockerfile_path.display()); // Log path used.
    info!("Using build context: {}", context_dir.display()); // Log context used (".").

    // --- Execute Docker Build ---
    // Print message to user indicating the start of the build.
    println!(
        "Building core environment image: {} (Using {}) (No Cache: {})...",
        full_image_tag, dockerfile_path_str, args.no_cache
    );
    // Call the shared Docker build utility function.
    docker::build_image(
        &full_image_tag,        // The final image tag.
        dockerfile_path_str,    // Relative path to Dockerfile within context.
        context_dir_str,        // Build context path (".").
        args.no_cache,          // Pass the no-cache flag.
    )
    .await // Await the async build process.
    .with_context(|| { // Add context if the build function returns an error.
        format!(
            "Failed to build core environment image '{}'",
            full_image_tag
        )
    })?;

    // --- Report Success ---
    // Log and print success message.
    info!(
        "Successfully built core environment image '{}'",
        full_image_tag
    );
    println!(
        "✅ Successfully built core environment image: {}",
        full_image_tag
    );

    Ok(()) // Indicate overall success.
}


// --- Unit Tests ---
/// Tests for the `env build` subcommand arguments and logic.
#[cfg(test)]
mod tests {
    use super::*; // Import items from parent module (build.rs)
    use crate::core::config::{Config, CoreEnvConfig}; // Import necessary config structs for mocking.
    use std::{env, fs};
    use tempfile::tempdir; // For creating temporary directories in tests.

    // Helper function to create a mock Config object.
    #[allow(dead_code)] // Allow if not used in all test variations
    fn mock_config(name: Option<&str>, tag: Option<&str>) -> Config {
        Config {
            core_env: CoreEnvConfig {
                image_name: name.map_or_else(String::new, String::from), // Use empty string if None
                image_tag: tag.map_or_else(String::new, String::from),   // Use empty string if None
                ..Default::default()
            },
            ..Default::default()
        }
    }

    // Helper function to set up a dummy Dockerfile in a temporary directory
    // and change the current working directory to it for the test duration.
    // Creates the file at the *new* expected path: presets/Dockerfile.devrs
    fn setup_dummy_dockerfile() -> Result<tempfile::TempDir> {
        let temp_dir = tempdir()?;
        let presets_dir = temp_dir.path().join("presets");
        fs::create_dir(&presets_dir)?; // Create presets subdir
        // Create the renamed dockerfile inside presets
        fs::write(
            presets_dir.join("Dockerfile.devrs"),
            "FROM scratch\nLABEL test=ok",
        )?;
        // Set CWD to the root of the temp dir (parent of presets)
        // **WARNING:** Affects process CWD. Use `serial_test` if needed.
        env::set_current_dir(temp_dir.path())?;
        Ok(temp_dir) // Return the guard to manage the temp directory's lifetime.
    }

    /// Test parsing of arguments like --no-cache and --stage.
    #[test]
    fn test_build_args_parsing() {
        // Test default args
        let args_default = BuildArgs::try_parse_from(["build"]).unwrap();
        assert!(!args_default.no_cache);
        assert!(args_default.stage.is_none());

        // Test flags enabled
        let args_flags =
            BuildArgs::try_parse_from(["build", "--no-cache", "--stage", "builder"]).unwrap();
        assert!(args_flags.no_cache);
        assert_eq!(args_flags.stage, Some("builder".to_string()));
    }

    /// Test the handler uses default image name/tag when config loading fails (mocked).
    #[tokio::test]
    #[ignore] // Ignored because it requires mocking config::load_config and docker::build_image.
    async fn test_handle_build_uses_default_on_config_fail() {
        // --- Setup ---
        let _temp_dir_guard = setup_dummy_dockerfile().expect("Test setup failed"); // Ensure Dockerfile exists at presets/Dockerfile.devrs

        // --- Mocking (Conceptual) ---
        // Mock `config::load_config()` to return `Err(anyhow!("Config load error"))`.
        // Mock `docker::build_image` to expect a call with the default tag "devrs-core-env:latest".

        // --- Test Execution ---
        let args = BuildArgs {
            no_cache: false,
            stage: None,
        };
        let result = handle_build(args).await;

        // --- Assertions ---
        assert!(result.is_ok()); // Expect overall success even if config load fails (uses defaults).
                                 // Verify (via mock) that `docker::build_image` was called with default tag.
                                 // Verify warning logged about config failure.
    }

    /// Test the handler uses image name/tag from loaded config when available and non-empty.
    #[tokio::test]
    #[ignore] // Ignored because it requires mocking config::load_config and docker::build_image.
    async fn test_handle_build_uses_config() {
        let _temp_dir_guard = setup_dummy_dockerfile().expect("Test setup failed");

        // --- Mocking (Conceptual) ---
        // Mock `config::load_config` -> Ok(mock_config(Some("custom-core"), Some("beta")))
        // Mock `docker::build_image` -> expect call with tag "custom-core:beta"

        // --- Test Execution ---
        let args = BuildArgs {
            no_cache: false,
            stage: None,
        };
        let result = handle_build(args).await;

        // --- Assertions ---
        assert!(result.is_ok());
        // Verify (via mock) that `docker::build_image` was called with tag "custom-core:beta".
    }

    /// Test the handler uses defaults if config values are empty strings.
    #[tokio::test]
    #[ignore] // Ignored because it requires mocking config::load_config and docker::build_image.
    async fn test_handle_build_uses_default_on_empty_config() {
        let _temp_dir_guard = setup_dummy_dockerfile().expect("Test setup failed");

        // --- Mocking (Conceptual) ---
        // Mock `config::load_config` -> Ok(mock_config(Some(""), Some(""))) // Empty strings
        // Mock `docker::build_image` -> expect call with default tag "devrs-core-env:latest"

        // --- Test Execution ---
        let args = BuildArgs {
            no_cache: false,
            stage: None,
        };
        let result = handle_build(args).await;

        // --- Assertions ---
        assert!(result.is_ok());
        // Verify (via mock) that `docker::build_image` was called with default tag.
    }

    /// Test the handler fails correctly if the Dockerfile (`presets/Dockerfile.devrs`) is missing.
    #[tokio::test]
    #[ignore] // Ignores because it requires mocking config::load_config.
    async fn test_handle_build_no_dockerfile() {
        // --- Setup ---
        // Create an empty temp directory and set it as CWD.
        let temp_dir = tempdir().unwrap();
        // **WARNING:** Affects process CWD. Use `serial_test` if needed.
        let original_cwd = env::current_dir().unwrap();
        env::set_current_dir(temp_dir.path()).expect("Failed to change CWD for test");
        // *Do not* create presets/Dockerfile.devrs

        // --- Mocking (Conceptual) ---
        // Mock `config::load_config` -> Ok(Config::default())

        // --- Test Execution ---
        let args = BuildArgs {
            no_cache: false,
            stage: None,
        };
        let result = handle_build(args).await;

        // --- Assertions ---
        assert!(result.is_err()); // Expect failure.
                                  // Check if the error message indicates the Dockerfile wasn't found.
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Dockerfile not found at expected path: presets/Dockerfile.devrs"));

        // --- Cleanup ---
        env::set_current_dir(original_cwd).unwrap(); // Restore original CWD
    }
}
