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
//! This image, defined by the main `Dockerfile` in the repository root, contains
//! all the common tools and dependencies needed for development across various projects
//! managed with DevRS.
//!
//! ## Architecture
//!
//! The command flow follows these steps:
//! 1. Parse command-line arguments (`BuildArgs`) using `clap`, specifically the `--no-cache` and `--stage` flags.
//! 2. Attempt to load the DevRS configuration (`core::config`) to retrieve the intended `image_name` and `image_tag` for the core environment. Fallback to hardcoded defaults ("devrs-env", "latest") if configuration loading fails or specific values are missing.
//! 3. Construct the full image tag (e.g., "my-custom-env:beta", "devrs-env:latest"). Validate that the resulting name and tag are not empty.
//! 4. Define the expected location of the `Dockerfile` (repository root) and the build context (repository root, represented as ".").
//! 5. Validate that the `Dockerfile` exists at the expected path. **Assumption:** This command is run from the DevRS repository root directory.
//! 6. Call the shared Docker utility function `common::docker::build_image` with the final image tag, Dockerfile path, context path, and cache options.
//! 7. Stream build output from Docker to the console.
//! 8. Report final success or failure.
//!
//! ## Examples
//!
//! Usage examples:
//!
//! ```bash
//! # Build the core environment image using defaults and cache
//! devrs env build
//!
//! # Build without using Docker's layer cache
//! devrs env build --no-cache
//!
//! # Build only up to a specific stage in the Dockerfile (NOTE: Currently informational only)
//! devrs env build --stage user-setup
//! ```
//!
//! The command builds the single, shared core development environment image, not project-specific application images (which are handled by `devrs container build`).
//!
use crate::common::docker; // Access shared Docker utilities (specifically build_image).
use crate::core::config; // Access configuration loading.
use crate::core::error::Result; // Standard Result type for error handling.
use anyhow::Context; // For adding context to errors.
use clap::Parser; // For parsing command-line arguments.
use std::path::Path; // For working with filesystem paths (Dockerfile location).
use tracing::{debug, info, warn}; // Logging framework utilities.

/// # Environment Build Arguments (`BuildArgs`)
///
/// Defines the command-line arguments accepted by the `devrs env build` subcommand.
/// Uses the `clap` crate for parsing and validation.
#[derive(Parser, Debug)]
pub struct BuildArgs {
    /// Optional: If set, instructs Docker to build the image without using its layer cache.
    /// This forces all steps in the Dockerfile to be re-executed, which can be useful
    /// for ensuring updates or debugging build issues, but is generally slower.
    #[arg(long)] // Define as `--no-cache`.
    no_cache: bool,

    /// Optional: Specifies a target stage to build up to within a multi-stage Dockerfile.
    /// Example: `--stage base-tools`.
    /// **Note:** While the argument is parsed, the underlying build logic in `common::docker::build_image`
    /// does not currently implement the `--target` flag for Docker builds. This option is currently informational.
    #[arg(long)] // Define as `--stage <STAGE_NAME>`.
    stage: Option<String>,
}

/// # Handle Environment Build Command (`handle_build`)
///
/// The main asynchronous handler function for the `devrs env build` command.
/// It orchestrates the process of building the core development environment Docker image
/// based on the main `Dockerfile` in the repository root and configuration settings.
///
/// ## Workflow:
/// 1.  Logs the start and parsed arguments. Notes if `--stage` is used but not fully implemented.
/// 2.  Defines hardcoded default values for the image name (`devrs-env`) and tag (`latest`).
/// 3.  Attempts to load the DevRS configuration using `core::config::load_config`.
/// 4.  If config loads successfully, it uses the `core_env.image_name` and `core_env.image_tag` from the config,
///     *unless* those values are empty strings, in which case it falls back to the hardcoded defaults.
/// 5.  If config loading fails, logs a warning and proceeds using the hardcoded defaults.
/// 6.  Constructs the full image tag string (e.g., `image_name:image_tag`).
/// 7.  Validates that neither the final `image_name` nor `image_tag` is empty.
/// 8.  Defines the expected relative path to the Dockerfile ("Dockerfile") and the build context directory (".").
/// 9.  Validates that the "Dockerfile" exists and is a file in the current working directory (assumed to be the repository root).
/// 10. Calls the shared `common::docker::build_image` function, providing the full image tag, Dockerfile path ("Dockerfile"),
///     context path ("."), and the `no_cache` flag.
/// 11. Streams Docker build output to the console.
/// 12. Logs and prints a success message upon completion.
///
/// ## Arguments
///
/// * `args`: The parsed `BuildArgs` struct containing the command-line options (`no_cache`, `stage`).
///
/// ## Returns
///
/// * `Result<()>`: Returns `Ok(())` on successful image build.
/// * `Err`: Returns an `Err` if configuration loading fails critically (e.g., invalid TOML),
///   if the Dockerfile is not found, or if the Docker build process itself fails.
pub async fn handle_build(args: BuildArgs) -> Result<()> {
    info!("Handling env build command..."); // Log entry point.
    debug!("Build args: {:?}", args); // Log parsed arguments.

    // Check if the --stage argument was used and warn that it's not fully implemented yet.
    if args.stage.is_some() {
        warn!("The --stage argument is not fully implemented in the build process yet.");
    }

    // --- Determine Image Name and Tag ---
    // Define default values used if config loading fails or values are empty.
    let default_image_name = "devrs-env"; // Default name for the core environment image.
    let default_image_tag = "latest"; // Default tag.

    // Initialize with defaults.
    let mut image_name = default_image_name.to_string();
    let mut image_tag = default_image_tag.to_string();

    // Attempt to load configuration.
    match config::load_config() { //
        Ok(cfg) => {
            // Config loaded successfully.
            info!("Successfully loaded configuration.");
            // Use the configured image name *only if it's not empty*.
            if !cfg.core_env.image_name.is_empty() {
                image_name = cfg.core_env.image_name;
            } else {
                info!(
                    "Loaded config core_env.image_name is empty, using default '{}'",
                    default_image_name
                );
            }
            // Use the configured image tag *only if it's not empty*.
            if !cfg.core_env.image_tag.is_empty() {
                image_tag = cfg.core_env.image_tag;
            } else {
                info!(
                    "Loaded config core_env.image_tag is empty, using default '{}'",
                    default_image_tag
                );
            }
        }
        Err(e) => {
            // Config loading failed. Log a warning but continue with defaults.
            // `load_config` generally only returns critical errors (like parse errors),
            // not file-not-found errors (which it handles internally).
            warn!("Could not load configuration ({}). Proceeding with default image name '{}' and tag '{}'.", e, default_image_name, default_image_tag);
        }
    }

    // Construct the full image tag string (e.g., "image_name:image_tag").
    let full_image_tag = format!("{}:{}", image_name, image_tag);
    info!("Using image target: {}", full_image_tag); // Log the final tag being used.

    // --- Sanity Check ---
    // Ensure that after processing config/defaults, we don't have an empty name or tag.
    if image_name.is_empty() || image_tag.is_empty() {
        anyhow::bail!( // Use anyhow::bail! for a direct error return.
            "Internal error: image name ('{}') or tag ('{}') is empty after config processing. Cannot build image ':'.",
            image_name, image_tag
        );
    }

    // --- Define Build Paths ---
    // Define the expected relative path to the Dockerfile.
    let dockerfile_path_str = "Dockerfile";
    // Define the build context directory (always the current directory, represented as ".").
    let context_dir_str = ".";
    // Create Path objects for validation.
    let dockerfile_path = Path::new(dockerfile_path_str);
    let context_dir = Path::new(context_dir_str); // Represents current directory.

    // --- Validate Dockerfile Existence ---
    // Check if the Dockerfile exists and is a file in the current directory.
    // This assumes the command is run from the repository root.
    if !dockerfile_path.exists() || !dockerfile_path.is_file() {
        anyhow::bail!(
            "Core environment Dockerfile not found at expected path: '{}'. Please run this command from the root of the devrs repository.",
            dockerfile_path.display()
        );
    }
    info!("Using Dockerfile: {}", dockerfile_path.display()); // Log path used.
    info!("Using build context: {}", context_dir.display()); // Log context used (".").

    // --- Execute Docker Build ---
    // Print message to user indicating the start of the build.
    println!(
        "Building core environment image: {} (No Cache: {})...",
        full_image_tag, args.no_cache
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
// Focus on argument parsing and tag generation logic.
// Testing the actual build requires mocking config loading and Docker API calls.
#[cfg(test)]
mod tests {
    use super::*;
    // May need config structs for mocking: use crate::config::{Config, CoreEnvConfig};
    use std::{env, fs};
    use tempfile::tempdir; // For creating temporary directories in tests.

    // Helper to set up a dummy Dockerfile in a temporary directory
    // and change the current working directory to it for the test duration.
    fn setup_dummy_dockerfile() -> tempfile::TempDir {
        let temp_dir = tempdir().unwrap();
        fs::write( // Create a minimal Dockerfile.
            temp_dir.path().join("Dockerfile"),
            "FROM scratch\nCMD echo hello",
        )
        .unwrap();
        // Store original CWD to restore later (important if tests run in parallel).
        // let _original_cwd = env::current_dir().unwrap(); // Store original CWD. This isn't easily restored.
        // Change CWD *for this test*. Relies on tests running serially or using a crate for CWD isolation.
        env::set_current_dir(temp_dir.path()).expect("Failed to change CWD for test");
        temp_dir // Return the guard to manage the temp directory's lifetime.
    }

    // Test the handler uses default image name/tag when config loading fails (mocked).
    #[tokio::test]
    #[ignore] // Ignored because it requires mocking config::load_config and docker::build_image, and manipulates CWD.
    async fn test_handle_build_uses_default() {
        // Setup: Ensure Dockerfile exists in a temporary CWD.
        let _temp_dir_guard = setup_dummy_dockerfile(); // CWD is now temp_dir

        // --- Mocking Setup (Conceptual) ---
        // Mock `config::load_config()` to return `Err(...)`.
        // Mock `docker::build_image` to expect a call with the default tag "devrs-env:latest".

        // --- Test Execution ---
        let args = BuildArgs {
            no_cache: false,
            stage: None,
        };
        let result = handle_build(args).await;

        // --- Assertions ---
        assert!(result.is_ok()); // Expect overall success even if config load fails (uses defaults).
        // Verify (via mock) that `docker::build_image` was called with tag "devrs-env:latest".
    }

    // Test the handler uses image name/tag from loaded config when available.
    #[tokio::test]
    #[ignore] // Ignored because it requires mocking config::load_config and docker::build_image, and manipulates CWD.
    async fn test_handle_build_uses_config() {
        let _temp_dir_guard = setup_dummy_dockerfile(); // Setup env.

        // --- Mocking Setup (Conceptual) ---
        // Mock `config::load_config()` to return `Ok(Config { core_env: CoreEnvConfig { image_name: "custom-name".into(), image_tag: "custom-tag".into(), .. }, .. })`.
        // Mock `docker::build_image` to expect a call with the tag "custom-name:custom-tag".

        // --- Test Execution ---
        let args = BuildArgs {
            no_cache: false,
            stage: None,
        };
        let result = handle_build(args).await;

        // --- Assertions ---
        assert!(result.is_ok());
        // Verify (via mock) that `docker::build_image` was called with tag "custom-name:custom-tag".
    }

    // Add test case for empty strings in config if needed (should fallback to defaults).

    // Test the handler fails correctly if the Dockerfile is missing.
    #[tokio::test]
    #[ignore] // Ignores because it requires mocking config::load_config and manipulates CWD.
    async fn test_handle_build_no_dockerfile_fallback() {
        // Setup: Create an empty temp directory and set it as CWD.
        let temp_dir = tempdir().unwrap();
        // let _original_cwd = env::current_dir().unwrap(); // Store original.
        env::set_current_dir(temp_dir.path()).expect("Failed to change CWD for test");

        // --- Mocking Setup (Conceptual) ---
        // Mock `config::load_config` to return Ok(Default::default()) or Err.

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
            .contains("Dockerfile not found"));

        // Restore CWD if possible/needed: env::set_current_dir(_original_cwd).unwrap();
    }
}
