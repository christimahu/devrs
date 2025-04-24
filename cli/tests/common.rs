//! # DevRS CLI Integration Test Common Helpers
//!
//! File: cli/tests/common.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! ## Overview
//!
//! This module provides shared utility functions and re-exports common crates
//! used across multiple integration test files (`blueprint.rs`, `container.rs`, etc.).
//! This avoids code duplication in the test suite.
//!
//! Integration tests are located in the `cli/tests/` directory and each `.rs` file
//! in that directory (that isn't a module like this one) is compiled as a separate
//! test crate linked against the main `devrs` binary crate.
//!

// Allow potentially unused code in this common module, as different test files might use different helpers.
#![allow(dead_code)]

// Re-export common crates/modules needed by multiple test files
pub use assert_cmd::Command;
// Note: predicates and tempfile are no longer re-exported from here.
// Individual test files should import them directly if needed using:
// use predicates::prelude::*;
// use tempfile::tempdir; // or other tempfile items

/// # Get DevRS Command (`devrs_cmd`)
///
/// Helper function to create an `assert_cmd::Command` instance pointing to the
/// compiled `devrs` binary target for the current test run.
///
/// This ensures tests execute the correct binary being built.
///
/// ## Panics
/// Panics if the `devrs` binary cannot be found via `Command::cargo_bin`.
///
/// ## Returns
/// * `Command` - An `assert_cmd::Command` ready to have arguments added and assertions run.
pub fn devrs_cmd() -> Command {
    Command::cargo_bin("devrs").expect("Failed to find devrs binary for testing")
}

// Add any other common setup functions or test helpers here in the future.
// For example:
// pub fn setup_temp_config_dir() -> tempfile::TempDir { /* ... */ }
// pub fn setup_mock_blueprint(name: &str) -> tempfile::TempDir { /* ... */ }
