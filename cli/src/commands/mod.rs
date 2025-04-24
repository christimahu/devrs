//! # DevRS Command Modules
//!
//! File: cli/src/commands/mod.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module aggregates all top-level command groups that comprise the DevRS CLI.
//! It serves as the central point for importing and re-exporting command modules
//! to make them accessible to the main application entry point (`main.rs`).
//!
//! ## Architecture
//!
//! The commands follow a hierarchical structure:
//! - Top-level modules represent command groups (e.g., `env`, `container`)
//! - Each group contains subcommands in their own modules or files
//! - All modules are made public for access from `main.rs`
//!
//! ## Command Groups
//!
//! - `blueprint`: Project template management commands
//! - `container`: Application container management commands
//! - `env`: Core development environment commands
//! - `setup`: Host system configuration commands
//! - `srv`: HTTP file server commands
//!
//! Each command group defines its own arguments structure and handler function
//! to process those arguments and implement the command's functionality.
//!

/// Command group for managing project templates (blueprints). Includes subcommands like `list`, `create`, `info`.
pub mod blueprint;
/// Command group for managing application-specific Docker containers. Includes subcommands like `build`, `run`, `logs`, `rm`.
pub mod container;
/// Command group for managing the core development environment container. Includes subcommands like `build`, `shell`, `exec`, `status`.
pub mod env;
/// Command group for setting up the host system with necessary dependencies and configurations. Includes subcommands like `all`, `nvim`, `shell`.
pub mod setup;
/// Command group for the static HTTP file server. Includes configuration and server logic.
pub mod srv;

// Note regarding subcommand declarations:
// Subcommands (like `build` within `container`, or `list` within `blueprint`)
// are declared within their respective parent module's `mod.rs` file.
// They are *not* declared here at the top level of the `commands` module.
// Example: `mod build;` exists within `commands/container/mod.rs`, not here.
