//! # DevRS Neovim Setup Handler
//!
//! File: cli/src/commands/setup/nvim.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs setup nvim` subcommand. It configures
//! the user's Neovim setup to use the configuration provided within the DevRS
//! repository (`presets/init.lua`), installs the Packer plugin manager if needed,
//! and optionally runs `PackerSync` to install Neovim plugins.
//!
//! ## Architecture
//!
//! 1.  Checks if the `nvim` and `git` commands are available on the host system using internal helpers (`check_command_exists`).
//! 2.  Determines the source path for the DevRS `init.lua` relative to the repository root (found using `find_repo_root`).
//! 3.  Determines the standard target path for the user's Neovim config (`~/.config/nvim/init.lua`) using the `directories` crate.
//! 4.  Ensures the target directory (`~/.config/nvim/`) exists using `fsio::ensure_dir_exists`.
//! 5.  Creates a symbolic link from the target path to the source `init.lua` using `fslinks::create_symlink`, which handles backups automatically. The `--force` flag ensures the link is recreated if it exists but is incorrect, or simply if forced.
//! 6.  Determines the installation path for the Packer plugin manager (`~/.local/share/nvim/site/pack/packer/start/packer.nvim`) using the `directories` crate.
//! 7.  Checks if the Packer directory exists. If it doesn't exist or if `--force` is specified, it attempts to clone the Packer repository from GitHub using the internal `run_external_command` helper for `git`.
//! 8.  If the `--skip-plugins` flag is *not* specified, it attempts to run `nvim --headless +PackerSync +qa` using the internal `run_external_command` helper to install/update plugins. Runs this step if `--force` was used as well, unless explicitly skipped.
//!
//! ## Error Handling
//! The command will fail if `nvim` or `git` are not found, if required paths cannot be determined, if filesystem operations fail (directory creation, symlinking, git clone), or if external commands (`git`, `nvim`) exit with non-zero status codes. Errors from external commands are wrapped in `DevrsError::ExternalCommand`. Failures during PackerSync (if run) are currently treated as warnings.
//!
//! ## Usage
//!
//! ```bash
//! # Must be run from the root of the devrs repository clone
//! cd /path/to/devrs
//!
//! # Basic Neovim setup (symlink config, install Packer, sync plugins)
//! devrs setup nvim
//!
//! # Force re-symlink, re-clone Packer, and re-run PackerSync
//! devrs setup nvim --force
//!
//! # Setup without installing/syncing plugins
//! devrs setup nvim --skip-plugins
//! ```
//!
use crate::{
    commands::setup::find_repo_root, // Use shared helper from setup::mod
    common::fs::{io as fsio, links as fslinks}, // Filesystem utilities
    core::error::{DevrsError, Result}, // Standard Result and error types
};
use anyhow::{anyhow, bail, Context}; // Error handling utilities
use clap::Parser; // Argument parsing
use directories::BaseDirs; // For finding user data/config directories
use std::{
    io::Write as IoWrite, // Bring trait into scope for stdout flush
    path::Path, // Path manipulation
    process::{Command, Stdio}, // For running external commands (git, nvim)
};
use tracing::{debug, error, info, warn}; // Logging

/// # Neovim Setup Arguments (`NvimArgs`)
///
/// Defines arguments for the `devrs setup nvim` subcommand, allowing control
/// over forcing actions and skipping plugin synchronization.
#[derive(Parser, Debug, Default)] // Default derive needed for setup all
#[command(
    about = "Configure Neovim to use DevRS settings and install Packer/plugins",
    long_about = "Sets up Neovim by symlinking the init.lua from the DevRS repository's 'presets' directory,\n\
                  installing the Packer plugin manager (packer.nvim), and optionally\n\
                  running PackerSync to install configured plugins."
)]
pub struct NvimArgs {
    /// Force setup steps: re-symlink init.lua (backing up existing), re-clone Packer, re-run PackerSync.
    #[arg(long, short)] // Define as `--force` or `-f`
    pub(crate) force: bool, // Made pub(crate) for access from all.rs
    /// Skip the Packer plugin installation step (PackerSync). Packer itself will still be installed if missing or forced.
    #[arg(long)] // Define as `--skip-plugins`
    pub(crate) skip_plugins: bool, // Made pub(crate) for access from all.rs
}

// --- Constants ---
/// URL for the Packer plugin manager repository.
const PACKER_REPO_URL: &str = "https://github.com/wbthomason/packer.nvim";

/// # Handle Neovim Setup Command (`handle_nvim`)
///
/// Asynchronous handler for `devrs setup nvim`. Performs Neovim configuration linking,
/// Packer installation, and optional plugin syncing based on provided arguments.
///
/// ## Arguments
///
/// * `args`: The parsed `NvimArgs` containing `--force` and `--skip-plugins` flags.
///
/// ## Returns
///
/// * `Result<()>`: `Ok(())` on successful setup.
/// * `Err`: If `nvim` or `git` are not found, paths are invalid, or external commands fail.
pub async fn handle_nvim(args: NvimArgs) -> Result<()> {
    info!(
        "Handling setup nvim command (Force: {}, SkipPlugins: {})...",
        args.force, args.skip_plugins
    );

    // --- 1. Check Dependencies (nvim and git) ---
    print!("Checking for 'nvim' command... ");
    std::io::stdout().flush().context("Failed to flush stdout")?;
    if !check_command_exists("nvim").await? {
        println!("Missing.");
        bail!("'nvim' command not found in PATH. Please install Neovim first.");
    }
    println!("Found.");

    print!("Checking for 'git' command... ");
    std::io::stdout().flush().context("Failed to flush stdout")?;
    if !check_command_exists("git").await? {
        println!("Missing.");
        bail!("'git' command not found in PATH. Git is required to install Packer.");
    }
    println!("Found.");

    // --- 2. Define Paths ---
    // Find the repository root to locate source files.
    let repo_root = find_repo_root()?;
    let presets_dir = repo_root.join("presets"); // Use the new 'presets' folder name
    let source_init_lua = presets_dir.join("init.lua");

    // Determine standard Neovim config directory (~/.config/nvim).
    let Some(base_dirs) = BaseDirs::new() else {
        bail!("Could not determine user's base directories (needed for Neovim config/data paths).");
    };
    // Standard XDG config location: ~/.config/nvim/
    let target_nvim_dir = base_dirs.config_dir().join("nvim");
    let target_init_lua = target_nvim_dir.join("init.lua");

    // Determine standard Packer installation path (~/.local/share/nvim/site/pack/packer/start/packer.nvim).
    let packer_dir = base_dirs
        .data_local_dir() // Usually ~/.local/share
        .join("nvim/site/pack/packer/start/packer.nvim"); // Full path including packer.nvim dir

    // Log the determined paths for debugging.
    debug!("Repo root: {}", repo_root.display());
    debug!("Source init.lua: {}", source_init_lua.display());
    debug!("Target nvim dir: {}", target_nvim_dir.display());
    debug!("Target init.lua: {}", target_init_lua.display());
    debug!("Packer install path: {}", packer_dir.display());

    // --- 3. Ensure Target Directory and Symlink Config ---
    // Ensure ~/.config/nvim directory exists.
    println!(
        "Ensuring Neovim config directory exists: {}",
        target_nvim_dir.display()
    );
    fsio::ensure_dir_exists(&target_nvim_dir).with_context(|| {
        format!(
            "Failed to ensure Neovim config directory exists at {}",
            target_nvim_dir.display()
        )
    })?;

    // Check if the source init.lua exists in the repository's presets directory.
    if !source_init_lua.is_file() {
        bail!(
            "DevRS Neovim config ('presets/init.lua') not found in repository at {}",
            source_init_lua.display()
        );
    }

    // Create symlink from target (~/.config/nvim/init.lua) to source (presets/init.lua).
    // The `create_symlink` helper handles backups of existing files at the target location automatically.
    println!(
        "Symlinking Neovim configuration from {} to {}...",
        source_init_lua.display(),
        target_init_lua.display()
    );
    if args.force && target_init_lua.symlink_metadata().is_ok() {
         warn!("--force specified, existing target at {} will be backed up if not the correct link or if it's not a link.", target_init_lua.display());
         // Let create_symlink handle the backup/replace logic.
    }
    fslinks::create_symlink(&source_init_lua, &target_init_lua).with_context(|| {
        format!(
            "Failed to symlink Neovim config from {} to {}",
            source_init_lua.display(),
            target_init_lua.display()
        )
    })?;
    println!("✅ Neovim config symlinked successfully.");

    // --- 4. Install Packer Plugin Manager ---
    // Check if the Packer directory exists.
    let packer_exists = packer_dir.is_dir(); // Check if it's a directory
    if !packer_exists || args.force {
        if args.force && packer_exists {
            warn!("--force specified, removing existing Packer directory...");
            // Attempt to remove the existing directory first when forcing.
            if let Err(e) = std::fs::remove_dir_all(&packer_dir) {
                // Log error but proceed, git clone might handle it or fail clearly.
                warn!(
                    "Failed to remove existing Packer directory at {}: {}. Continuing with clone attempt.",
                    packer_dir.display(), e
                );
            }
        }
        println!("Packer plugin manager not found or --force used. Installing Packer via git clone...");
        // Ensure parent directories exist for the clone target (e.g., .../packer/start/).
        if let Some(packer_parent) = packer_dir.parent() {
            fsio::ensure_dir_exists(packer_parent).with_context(|| {
                format!(
                    "Failed to ensure Packer parent directory exists at {}",
                    packer_parent.display()
                )
            })?;
        } else {
            // This case should be unlikely given the standard path structure.
            bail!("Could not determine parent directory for Packer installation path: {}", packer_dir.display());
        }

        // Run git clone command to install Packer using internal helper.
        // Convert PathBuf to &str for the command argument safely.
        let Some(packer_dir_str) = packer_dir.to_str() else {
             bail!("Packer installation path is not valid UTF-8: {}", packer_dir.display());
         };
        // Define arguments for git clone.
        let clone_args = &["clone", "--depth", "1", PACKER_REPO_URL, packer_dir_str][..];

        // Execute the git clone command.
        run_external_command("git", clone_args, None)
            .await
            .with_context(|| format!("Failed to clone Packer repository into {}", packer_dir_str))?;

        println!("✅ Packer installed successfully.");
    } else {
        println!("Packer plugin manager already installed.");
    }

    // --- 5. Install/Sync Plugins via PackerSync ---
    // Run PackerSync unless explicitly skipped by the user (--skip-plugins).
    // Also run if --force was specified, to ensure plugins are up-to-date.
    if !args.skip_plugins {
        println!("Running PackerSync to install/update Neovim plugins...");
        // Run nvim headlessly to execute PackerSync and quit automatically.
        let nvim_sync_args = &["--headless", "+PackerSync", "+qa"][..];
        match run_external_command("nvim", nvim_sync_args, None).await {
            Ok(_) => {
                println!("✅ PackerSync completed successfully.");
            }
            Err(e) => {
                // Log the error.
                error!("PackerSync command failed: {}", e);
                // Warn the user but don't fail the entire setup; they can run it manually.
                warn!("Failed to run PackerSync automatically. Please check Neovim output or run manually.");
                println!("⚠️ PackerSync failed or reported errors. Please run Neovim and execute `:PackerSync` manually to check plugin status.");
            }
        }
    } else {
        info!("Skipping plugin installation (--skip-plugins specified).");
        println!("Skipped plugin installation via PackerSync.");
        println!("Run Neovim and execute `:PackerSync` manually if needed.");
    }

    println!("Neovim setup process finished.");
    Ok(()) // Overall success.
}

/// # Check Command Existence (`check_command_exists`)
///
/// Internal helper to check if a command is available in the system's PATH
/// by attempting to run it with a version flag.
///
/// ## Arguments
/// * `cmd_name`: The name of the command (e.g., "nvim", "git").
///
/// ## Returns
/// * `Result<bool>`: `Ok(true)` if found, `Ok(false)` if not found (ErrorKind::NotFound).
/// * `Err`: On other execution errors.
async fn check_command_exists(cmd_name: &str) -> Result<bool> {
    info!("Performing basic check for command: {}", cmd_name);
    let mut command = Command::new(cmd_name);
    command.arg("--version");
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());

    debug!("Running check: {} --version", cmd_name);
    match command.status() {
        Ok(status) => {
            debug!("Check status for '{}': {}", cmd_name, status);
            // Consider command found if it executed, even if --version fails (e.g., returns non-zero)
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            debug!("Command '{}' not found (ErrorKind::NotFound).", cmd_name);
            Ok(false)
        }
        Err(e) => Err(anyhow::Error::new(e)
            .context(format!("Failed to execute command check for '{}'", cmd_name))),
    }
}

/// # Run External Command (`run_external_command`)
///
/// Executes an external command (like `git` or `nvim`) with specified arguments,
/// inheriting standard input, output, and error streams.
///
/// ## Arguments
///
/// * `program`: The name of the command to execute.
/// * `args`: A slice of string slices representing the arguments.
/// * `cwd`: An optional path to set the working directory.
///
/// ## Returns
///
/// * `Result<()>`: `Ok(())` if the command exits successfully (status code 0).
/// * `Err`: If the command cannot be executed or exits non-zero, returning `DevrsError::ExternalCommand`.
async fn run_external_command(
    program: &str,
    args: &[&str],
    cwd: Option<&Path>,
) -> Result<()> {
    info!("Executing command: {} {:?}", program, args);
    let mut command = tokio::process::Command::new(program);
    command.args(args);
    if let Some(dir) = cwd {
        command.current_dir(dir);
        info!("Setting CWD for command to {}", dir.display());
    }

    // Inherit stdio so user sees output (e.g., git clone progress, PackerSync output).
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());
    command.stdin(Stdio::inherit());

    // Execute the command asynchronously and wait for its status.
    let status = command
        .status()
        .await
        .with_context(|| format!("Failed to execute command '{}'. Is it installed and in PATH?", program))?;

    // Check if the command executed successfully (exit code 0).
    if !status.success() {
        let exit_code = status.code().map_or("?".to_string(), |c| c.to_string());
        error!(
            "Command '{} {:?}' failed with exit code {}",
            program, args, exit_code
        );
        // Return a specific DevrsError indicating external command failure.
        return Err(anyhow!(DevrsError::ExternalCommand {
            cmd: format!("{} {}", program, args.join(" ")),
            status: exit_code,
            output: "Command failed. See terminal output above for details.".to_string(),
        }));
    }

    info!("Command '{} {:?}' completed successfully.", program, args);
    Ok(())
}


// --- Unit Tests ---
/// Tests for Neovim setup arguments and logic.
#[cfg(test)]
mod tests {
    use super::*; // Import items from parent module (nvim.rs)

    /// Test argument parsing for flags like --force and --skip-plugins.
    #[test]
    fn test_nvim_args_parsing() {
        // Test default arguments.
        let args_default = NvimArgs::try_parse_from(&["nvim"]).unwrap();
        assert!(!args_default.force, "Default force should be false");
        assert!(
            !args_default.skip_plugins,
            "Default skip_plugins should be false"
        );

        // Test parsing with flags enabled.
        let args_flags =
            NvimArgs::try_parse_from(&["nvim", "--force", "--skip-plugins"]).unwrap();
        assert!(args_flags.force, "force should be true");
        assert!(
            args_flags.skip_plugins,
            "skip_plugins should be true"
        );

        // Test parsing with short flag -f for force.
        let args_force_only = NvimArgs::try_parse_from(&["nvim", "-f"]).unwrap();
        assert!(args_force_only.force, "force should be true via -f");
        assert!(
            !args_force_only.skip_plugins,
            "skip_plugins should be false with only -f"
        );
    }

    /// Test the check for command existence using the implemented helper.
    /// Relies on `git` being available in the test environment.
    #[tokio::test]
    #[ignore] // Ignored because it relies on external `git` command presence.
    async fn test_check_command_exists_logic() {
        // Test for git, required by nvim setup for Packer.
        match check_command_exists("git").await {
            Ok(found) => assert!(found, "Expected 'git' to be found in PATH for test."),
            Err(e) => panic!("git check failed unexpectedly: {}", e),
        }
        // Test for nvim (might or might not be present)
        // let nvim_found = check_command_exists("nvim").await.unwrap_or(false);
        // println!("Nvim found in test env: {}", nvim_found); // Informational

        // Test for a non-existent command
        let non_existent_found = check_command_exists("nonexistent_devrs_cmd_0987").await.unwrap_or(true);
        assert!(!non_existent_found, "Non-existent command should not be found");

    }

    /// Test the helper function for running external commands (success case).
    #[tokio::test]
    async fn test_run_external_command_success() {
        // Use a simple command expected to succeed.
        let (program, args) = if cfg!(windows) {
            ("cmd", &["/C", "echo", "test"][..])
        } else {
            ("echo", &["test"][..])
        };
        let result = run_external_command(program, args, None).await;
        assert!(result.is_ok());
    }

    /// Test the helper function for running external commands (failure case - exit code).
    #[tokio::test]
    async fn test_run_external_command_fail_exit() {
        // Use a command that exists but exits non-zero.
        let (program, args) = if cfg!(windows) {
            ("cmd", &["/C", "exit", "1"][..])
        } else {
            ("sh", &["-c", "exit 1"][..])
        };
        let result = run_external_command(program, args, None).await;
        assert!(result.is_err());
        // Check if it's the specific ExternalCommand error with status "1".
        let err = result.unwrap_err();
        let devrs_err = err.downcast_ref::<DevrsError>();
        assert!(matches!(devrs_err, Some(DevrsError::ExternalCommand { status, .. }) if status == "1"));
    }

    /// Test the helper function for running external commands (failure case - not found).
    #[tokio::test]
    async fn test_run_external_command_fail_not_found() {
        let result = run_external_command("nonexistent_command_for_test_4321", &[], None).await;
        assert!(result.is_err());
        // Check if the error indicates execution failure (e.g., contains "Failed to execute").
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to execute command"));
    }


    /// Placeholder test for the main handler logic (`handle_nvim`).
    /// Requires extensive mocking of filesystem, BaseDirs, find_repo_root,
    /// and the external command helpers (`check_...`, `run_external_command`).
    #[tokio::test]
    #[ignore] // Requires extensive mocking.
    async fn test_handle_nvim_logic() {
        // --- Mocking/Setup (Conceptual) ---
        // Mock `check_command_exists` -> Ok(true) for nvim and git
        // Mock `find_repo_root` -> Ok(temp_dir_path)
        // Mock `BaseDirs::new()` and its methods -> return paths within temp dir
        // Mock `fsio::ensure_dir_exists` -> Ok(())
        // Mock `fslinks::create_symlink` -> Ok(())
        // Mock `packer_dir.exists()` -> false (first run)
        // Mock `run_external_command` for `git clone` -> Ok(())
        // Mock `run_external_command` for `nvim --headless...` -> Ok(())
        // Create dummy init.lua in mocked repo root/presets

        // --- Test Execution (Basic) ---
        let args = NvimArgs {
            force: false,
            skip_plugins: false,
        };
        let result = handle_nvim(args).await;

        // --- Assertions (Basic) ---
        assert!(result.is_ok());
        // Verify mocks were called in order: checks, ensure_dir, symlink, packer_exists(false), ensure_dir(parent), git_clone, nvim_headless.

        // --- Add more test cases ---
        // - Case: --skip-plugins (verify nvim_headless not called)
        // - Case: --force (verify symlink called, packer dir removed?, git_clone called, nvim_headless called)
        // - Case: Packer already exists (verify git_clone not called unless force=true)
        // - Case: Dependency checks fail -> Err
        // - Case: `git clone` helper returns Err -> Err
        // - Case: `nvim headless` helper returns Err -> Warns, but overall Ok()
    }
}
