//! # DevRS CLI Setup Integration Tests
//!
//! File: cli/tests/setup.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! Integration tests for the `devrs setup` subcommand group (`deps`, `integrate`, `nvim`, `all`).
//! These tests verify the CLI behavior for setting up the host environment by executing
//! the compiled `devrs` binary.
//!
//! **Note on Test Environment:** These tests interact with the filesystem and may
//! execute external commands (`git`, `nvim`, dependency checks). They generally
//! require a specific environment to pass reliably:
//!   - Docker, Git, and Neovim must be installed and accessible in the system's PATH.
//!   - The tests must be executed from the **root directory** of the `devrs` repository clone,
//!     as the setup commands need to locate files relative to this root.
//!   - Tests involving significant side effects (`nvim`, `all`) only perform basic
//!     assertions (like command success exit code) due to the complexity of mocking
//!     or verifying filesystem changes and external processes within this test suite.
//!

// Use helpers from the common test module (e.g., devrs_cmd)
mod common;
use common::*;
// Import assertion library features.
use predicates::prelude::*;

/// # Test: `devrs setup deps`
///
/// Verifies basic invocation and output of the `deps` subcommand.
/// Asserts that the command starts, prints its initial check message, and exits successfully.
/// Assumes required dependencies (`docker`, `git`, `nvim`) are present in the test environment's PATH.
#[test]
fn test_setup_deps() {
    // Requires `docker`, `git`, `nvim` to be in PATH for success.
    devrs_cmd()
        .args(["setup", "deps"]) // Use the correct subcommand name: deps
        .assert()
        .success() // Asserts exit code 0 (assumes deps are found)
        .stdout(predicate::str::contains(
            "Checking required host dependencies...", // Verify initial output
        ));
}

/// # Test: `devrs setup integrate`
///
/// Verifies basic invocation and output of the `integrate` subcommand.
/// This command should print instructions for sourcing shell functions.
/// The test checks for key phrases in the output, including the correct path segment.
/// Requires execution from the repository root.
#[test]
fn test_setup_integrate() {
    // This test assumes it's run from the repo root so find_repo_root works
    // and the path printed reflects the 'presets' directory.
    devrs_cmd()
        .args(["setup", "integrate"]) // Use the correct subcommand name: integrate
        .assert()
        .success() // Expect exit code 0
        .stdout(
                // Check for essential parts that are always present.
                predicate::str::contains("source") // Check for the 'source' keyword
                    .and(predicate::str::contains("presets/shell_functions")) // Check for the script path segment
                    .and(predicate::str::contains("manually add** the following")), // <<< Check for this more specific, stable phrase
            );
}

/// # Test: `devrs setup nvim`
///
/// Verifies basic invocation of the `nvim` subcommand.
/// **Limitation:** This is a basic test asserting only that the command executes
/// and returns a success exit code (0). It does **not** verify the complex side effects
/// like filesystem symlinking, `git clone` operations for Packer, or the success/output
/// of the `nvim --headless +PackerSync` command.
/// Requires `nvim` and `git` in PATH and execution from the repository root.
#[test]
fn test_setup_nvim() {
    // Assumes test runs from repo root and nvim/git are installed.
    // Only checks for successful exit code due to complex side effects.
    devrs_cmd().args(["setup", "nvim"]).assert().success();
}

/// # Test: `devrs setup all` (and default `devrs setup`)
///
/// Verifies basic invocation of the `all` subcommand (which is also the default
/// action for `devrs setup`).
/// **Limitation:** Similar to `test_setup_nvim`, this is a basic test asserting only
/// command success (exit code 0). It does **not** verify the completion or correctness
/// of all the underlying steps (dependency checks, config symlink, integrate output, nvim setup).
/// Requires all dependencies (`docker`, `git`, `nvim`) in PATH and execution from the repository root.
#[test]
fn test_setup_all() {
    // Assumes test runs from repo root and all dependencies are installed.
    // Only checks for successful exit code due to complex side effects.
    // Using ["setup"] implicitly tests the 'all' subcommand default.
    devrs_cmd().args(["setup"]).assert().success();
}
