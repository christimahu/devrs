//! # DevRS Environment Rebuild Handler
//!
//! File: cli/src/commands/env/rebuild.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs env rebuild` subcommand. Its purpose is to
//! provide a convenient way to completely refresh the **core development environment**.
//! This involves stopping and removing the existing core environment container (if one exists)
//! and then building a fresh Docker image based on the current `Dockerfile` and configuration.
//!
//! ## Architecture
//!
//! The command orchestrates several steps:
//! 1. Parse command-line arguments (`RebuildArgs`) using `clap`, including flags like `--no-cache`, `--name`, and the experimental `--with-plugins`.
//! 2. Load the DevRS configuration (`core::config`) to determine core environment settings (image name/tag, default container name).
//! 3. Determine the target core environment container name (using `--name` override or generating the default).
//! 4. Attempt to gracefully **stop** the existing container using `common::docker::lifecycle::stop_container`, handling "not found" errors non-fatally.
//! 5. Attempt to **remove** the existing container using `common::docker::lifecycle::remove_container`, handling "not found" errors non-fatally.
//! 6. Determine the image tag, Dockerfile path ("Dockerfile"), and build context (".") based on configuration and the assumption that the command is run from the repository root.
//! 7. Validate that the `Dockerfile` exists.
//! 8. **Build** the new core environment image using `common::docker::build_image`, passing the `no_cache` flag if specified.
//! 9. Report success and suggest the next step (`devrs env shell`).
//!
//! ## Usage
//!
//! ```bash
//! # Stop, remove, and rebuild the default core environment
//! devrs env rebuild
//!
//! # Rebuild without using Docker's layer cache
//! devrs env rebuild --no-cache
//!
//! # Rebuild and attempt experimental plugin update (requires Dockerfile support)
//! devrs env rebuild --with-plugins
//!
//! # Rebuild a specifically named core environment container/image
//! devrs env rebuild --name my-custom-env-instance
//! ```
//!
//! This command essentially combines `devrs env stop`, `devrs env prune` (implicitly, by removing the specific container), and `devrs env build` into a single operation for the core environment.
//!
use crate::{
    common::docker, // Access shared Docker utilities (stop, remove, build).
    core::{config, error::Result}, // Standard config loading and Result type.
    // Removed direct imports of build and stop handlers as they are not called directly.
};
use anyhow::Context; // For adding context to errors.
use clap::Parser; // For parsing command-line arguments.
use std::path::Path; // For working with filesystem paths (Dockerfile location).
use tracing::{info, warn}; // Logging framework utilities.

/// # Environment Rebuild Arguments (`RebuildArgs`)
///
/// Defines the command-line arguments accepted by the `devrs env rebuild` subcommand.
/// Uses the `clap` crate for parsing and validation.
#[derive(Parser, Debug)]
#[command(
    about = "Rebuild the core DevRS environment image and recreate the container",
    long_about = "Stops and removes the existing core environment container (if any),\n\
                  then rebuilds the image using the configuration.\n\
                  Note: --with-plugins functionality requires specific Dockerfile support."
)]
pub struct RebuildArgs {
    /// Optional: If set, instructs Docker to build the image without using its layer cache,
    /// forcing all steps in the Dockerfile to re-run.
    #[arg(long)] // Define as `--no-cache`.
    no_cache: bool,

    /// Optional: Specifies the exact name of the core environment container to stop/remove
    /// before rebuilding the image. If omitted, the default name derived from configuration
    /// (`<core_env.image_name>-instance`) is used. Note that the rebuilt image tag is
    /// always determined by the configuration, regardless of this argument.
    #[arg(long)] // Define as `--name <NAME>`.
    name: Option<String>,

    /// Optional: Experimental flag indicating an attempt should be made to update Neovim plugins
    /// during the image rebuild process.
    /// **WARNING:** This functionality is not fully implemented and depends heavily on
    /// specific stages or logic being present within the core environment `Dockerfile`
    /// (e.g., a stage that checks out configuration and runs `PackerSync`). It is not guaranteed
    /// to work with the default Dockerfile.
    #[arg(long)] // Define as `--with-plugins`.
    with_plugins: bool,
}

/// # Handle Environment Rebuild Command (`handle_rebuild`)
///
/// The main asynchronous handler function for the `devrs env rebuild` command.
/// It orchestrates stopping and removing the existing core environment container
/// (if it exists) and then building a fresh image based on the current `Dockerfile`.
///
/// ## Workflow:
/// 1. Logs the start and parsed arguments. Issues a warning if the experimental `--with-plugins` flag is used.
/// 2. Loads the DevRS configuration (`core::config`) to get core environment settings (image name/tag).
/// 3. Determines the target container name using `get_core_env_container_name` (honoring the `--name` override or using the default derived from config).
/// 4. Attempts to stop the determined container using `docker::lifecycle::stop_container` with a short timeout. Logs warnings on failure but continues (e.g., if container wasn't running or didn't exist).
/// 5. Attempts to remove the determined container using `docker::lifecycle::remove_container` with `force=true` (to handle stopped/exited states). Logs warnings on failure but continues (e.g., if container didn't exist).
/// 6. Retrieves the configured image name and tag from the loaded config.
/// 7. Constructs the full image tag string (e.g., `image_name:tag`).
/// 8. Defines the Dockerfile path ("Dockerfile") and context path (".") assuming execution from the repository root.
/// 9. Validates that the `Dockerfile` exists at the expected path.
/// 10. Calls the shared `common::docker::build_image` function with the full image tag, Dockerfile path, context path, and the `no_cache` flag.
/// 11. Prints a success message and suggests running `devrs env shell`.
///
/// ## Arguments
///
/// * `args`: The parsed `RebuildArgs` struct containing the command-line options.
///
/// ## Returns
///
/// * `Result<()>`: Returns `Ok(())` on successful rebuild.
/// * `Err`: Returns an `Err` if config loading fails, the Dockerfile is not found, or the Docker build process fails. Failures during the stop/remove phases are logged as warnings but do not typically cause the command to fail overall.
pub async fn handle_rebuild(args: RebuildArgs) -> Result<()> {
    info!("Handling env rebuild command..."); // Log entry point.
    // Log the specific arguments received.
    info!(
        "(Name: {:?}, NoCache: {}, WithPlugins: {})",
        args.name, args.no_cache, args.with_plugins
    );

    // Warn if the experimental --with-plugins flag is used.
    if args.with_plugins {
        warn!("The --with-plugins flag is experimental and requires specific Dockerfile setup.");
        warn!("Ensure your Dockerfile has a stage designed for plugin updates.");
        // Inform user that the core logic for this isn't fully implemented yet.
        println!("Proceeding with standard rebuild; plugin update logic is not fully implemented.");
        // Note: Implementing this would likely involve passing build arguments or targets
        // to the `docker::build_image` function, which currently doesn't support them.
    }

    // 1. Load configuration.
    let cfg = config::load_config().context("Failed to load DevRS configuration")?;

    // 2. Determine the name of the container to stop/remove.
    let container_name = args.name.clone().unwrap_or_else(|| {
        // If --name wasn't provided, generate the default name.
        let default_name = get_core_env_container_name(&cfg);
        info!("No specific name provided, using default: {}", default_name);
        default_name
    });

    println!("Rebuilding core environment '{}'...", container_name);

    // 3. Attempt to stop the existing container (best effort, ignore most errors).
    println!(
        "Attempting to stop existing container '{}' (if running)...",
        container_name
    );
    match docker::lifecycle::stop_container(&container_name, Some(5)).await { // Use short 5s timeout.
        Ok(()) => info!( // Log success or already stopped.
            "Container '{}' stopped or was already stopped.",
            container_name
        ),
        // Handle container not found specifically (it's okay for rebuild).
        Err(e)
            if e.downcast_ref::<crate::core::error::DevrsError>()
                .is_some_and(|de| {
                    matches!(de, crate::core::error::DevrsError::ContainerNotFound { .. })
                }) =>
        {
            warn!("Container '{}' not found during stop step.", container_name);
        }
        // Log other errors but don't fail the whole rebuild operation just for a stop error.
        Err(e) => warn!("Stop command failed: {}", e),
    }

    // 4. Attempt to remove the existing container (best effort, ignore most errors).
    println!(
        "Attempting to remove existing container '{}' (if exists)...",
        container_name
    );
    // Use force=true here to handle containers that might be stopped but not cleanly removable otherwise.
    match docker::lifecycle::remove_container(&container_name, true).await { //
        Ok(()) => info!( // Log success or already absent.
            "Container '{}' removed or was already absent.",
            container_name
        ),
         // Handle container not found specifically (it's okay for rebuild).
        Err(e)
            if e.downcast_ref::<crate::core::error::DevrsError>()
                .is_some_and(|de| {
                    matches!(de, crate::core::error::DevrsError::ContainerNotFound { .. })
                }) =>
        {
            warn!(
                "Container '{}' not found during removal step.",
                container_name
            );
        }
        // Log other errors but don't fail the whole rebuild operation just for a remove error.
        Err(e) => warn!("Remove command failed: {}", e),
    }

    // 5. Build the new image. This step *should* fail the command if it errors out.
    println!("Building the core environment image...");
    // Get image name and tag from the loaded (or default) config.
    let image_name = &cfg.core_env.image_name;
    let image_tag = &cfg.core_env.image_tag;
    let full_image_tag = format!("{}:{}", image_name, image_tag);

    // Define paths, assuming execution from repository root.
    let dockerfile_path_str = "Dockerfile";
    let context_dir_str = ".";
    let dockerfile_path = Path::new(dockerfile_path_str);

    // Validate Dockerfile existence.
    if !dockerfile_path.exists() || !dockerfile_path.is_file() {
        anyhow::bail!(
            "Core environment Dockerfile not found at expected path: '{}'. Please run this command from the root of the devrs repository.",
            dockerfile_path.display()
        );
    }
    info!("Using Dockerfile: {}", dockerfile_path.display());
    // Try to get canonical path for context logging, but don't fail if it errors.
    let context_display_path = Path::new(context_dir_str).canonicalize().map_or_else(
        |_| context_dir_str.to_string(), // Fallback to "." if canonicalize fails
        |p| p.display().to_string()
    );
    info!("Using build context: {}", context_display_path);

    // Call the shared build function.
    docker::build_image( //
        &full_image_tag,
        dockerfile_path_str, // Pass relative path string.
        context_dir_str,     // Pass context path string (".").
        args.no_cache,       // Pass no-cache flag.
    )
    .await // Await the async build.
    .with_context(|| { // Add context on error.
        format!(
            "Failed to build core environment image '{}'",
            full_image_tag
        )
    })?;

    // 6. Report overall success.
    println!(
        "✅ Environment rebuild process completed for '{}'.",
        container_name // Report using the container name determined earlier.
    );
    println!("Run 'devrs env shell' to start using the rebuilt environment.");

    Ok(()) // Indicate overall success.
}

/// # Get Core Environment Container Name (`get_core_env_container_name`)
///
/// Helper function to consistently derive the default name for the core development
/// environment container based on the image name from the configuration.
///
/// ## Logic:
/// Appends "-instance" to the configured `core_env.image_name`.
///
/// ## Arguments
///
/// * `cfg`: A reference (`&`) to the loaded `Config` struct.
///
/// ## Returns
///
/// * `String`: The derived default container name (e.g., "devrs-core-tools-instance").
fn get_core_env_container_name(cfg: &config::Config) -> String {
    // Use format! for simple string construction.
    format!("{}-instance", cfg.core_env.image_name)
}


// --- Unit Tests ---
// Focus on argument parsing. Testing handler logic requires mocking.
#[cfg(test)]
mod tests {
    use super::*;

    /// Test parsing arguments including optional flags like --name, --no-cache, --with-plugins.
    #[test]
    fn test_rebuild_args_parsing() {
        // Simulate `devrs env rebuild --name my-env --no-cache --with-plugins`
        let args = RebuildArgs::try_parse_from([
            "rebuild", // Command name context for clap.
            "--name",
            "my-env",
            "--no-cache",
            "--with-plugins",
        ])
        .unwrap(); // Expect parsing to succeed.

        // Verify parsed values.
        assert_eq!(args.name, Some("my-env".to_string()));
        assert!(args.no_cache);
        assert!(args.with_plugins);
    }

    /// Test parsing with only default flags.
     #[test]
    fn test_rebuild_args_parsing_defaults() {
         // Simulate `devrs env rebuild`
         let args = RebuildArgs::try_parse_from(["rebuild"]).unwrap();
         // Verify default values.
         assert!(args.name.is_none());
         assert!(!args.no_cache);
         assert!(!args.with_plugins);
     }


    // Note: Testing the `handle_rebuild` function's logic requires mocking:
    // 1. `config::load_config` -> To provide known core env settings.
    // 2. `common::docker::lifecycle::stop_container` -> To simulate stopping outcomes.
    // 3. `common::docker::lifecycle::remove_container` -> To simulate removal outcomes.
    // 4. `common::docker::build_image` -> To simulate build success/failure.
    // 5. Filesystem checks (`Dockerfile` existence).
    // Then, tests could verify the sequence of operations (stop->remove->build),
    // correct handling of errors during stop/remove (should log warning but continue),
    // correct arguments passed to `build_image`, and final result reporting.
    #[tokio::test]
    #[ignore] // Requires mocking config and docker calls.
    async fn test_handle_rebuild_logic() {
        // --- Mocking Setup (Conceptual) ---
        // Mock config loading.
        // Mock docker::stop_container, docker::remove_container, docker::build_image.
        // Ensure dummy Dockerfile exists if needed for validation step.

        // --- Execution ---
        let args = RebuildArgs {
            no_cache: true,
            name: Some("test-env".to_string()),
            with_plugins: false,
        };
        let result = handle_rebuild(args).await;

        // --- Assertions ---
        assert!(result.is_ok()); // Expect overall success if mocks are set up correctly.
        // Verify mocks show stop, rm, and build (from common::docker) were called in order.
        // Verify correct arguments passed to build_image (tag from config, no_cache=true).
    }
}
