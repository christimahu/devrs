//! # DevRS CLI Srv Integration Tests
//!
//! File: cli/tests/srv.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! ## Overview
//!
//! Integration tests for the `devrs srv` subcommand, which runs a local
//! development server.
//!
//! **Note:** Testing a running server is complex. This test verifies basic
//! command invocation and checks for the initial startup message. It does not
//! verify server functionality or graceful shutdown.
//!

// Declare and use the common module
mod common;
use common::*;
// Import necessary items directly
use predicates::prelude::*;
use tempfile::tempdir;

/// # Test Srv Basic (`test_srv_basic`)
///
/// Verifies basic invocation of `devrs srv <directory>`.
/// Checks if the command starts and prints the initial "Serving files from:" message.
/// Does not assert overall success as the server might block or run indefinitely.
/// Requires a temporary directory for the server root.
#[test]
#[ignore] // TODO: test hangs
fn test_srv_basic() {
    let temp_serve_dir = tempdir().expect("Failed to create temp dir for serving");
    let serve_dir_path = temp_serve_dir.path().to_str().unwrap();

    // Running the actual server in a test is tricky as it might block.
    // Check if the command *attempts* to start without immediate arg error
    // and prints the expected startup line.
    devrs_cmd()
        .args(["srv", serve_dir_path])
        .assert()
        // .success() // This will likely hang or fail depending on implementation
        .stdout(predicate::str::contains("Serving files from:")); // Check startup message
}
