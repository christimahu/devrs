//! # DevRS Common Utilities (`common`)
//!
//! File: cli/src/common/mod.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!

//! ## Overview
//!
//! This module serves as the root and organizational entry point for all shared,
//! common utility modules used throughout the DevRS CLI application. It aggregates
//! functionality related to cross-cutting concerns like filesystem operations,
//! Docker interactions, networking, process execution, system checks, UI elements,
//! and archive handling.
//!
//! By centralizing these utilities under the `common::` namespace, DevRS aims to
//! promote code reuse, maintain consistency, and provide clear separation between
//! command-specific logic (`commands::`) and core infrastructure (`core::`).
//!
//! ## Architecture
//!
//! The `common` module itself primarily consists of declarations (`pub mod`) for its
//! various submodules. Each submodule encapsulates a specific domain of utility functions:
//!
//! - **`archive`**: Utilities for creating and potentially extracting archive files (e.g., `.tar.gz`). Includes the `tar` submodule.
//! - **`docker`**: The main interface for interacting with the Docker daemon via the `bollard` crate. Handles images, containers, lifecycle, state, interaction, etc.
//! - **`fs`**: Foundational filesystem operations like reading/writing files, copying directories, ensuring directory existence, and managing symbolic links. Includes `io`, `copy`, `links`.
//! - **`network`**: *(Placeholder)* Intended for network-related utilities like IP detection and port checking.
//! - **`process`**: *(Placeholder)* Intended for executing external commands/processes and managing their output.
//! - **`system`**: *(Placeholder)* Intended for system-level inspection like shell detection or checking for required tools.
//! - **`ui`**: *(Placeholder)* Intended for terminal UI enhancements like progress bars, tables, and prompts.
//!
//! ## Usage
//!
//! Command handlers and other parts of the application import specific functionalities
//! directly from the required submodule within `common`.
//!
//! ```rust
//! // Example importing from different common submodules
//! use crate::common::{docker, fs, archive}; // Import the parent modules
//! use crate::core::error::Result;
//! use std::path::Path;
//!
//! # async fn run_example() -> Result<()> {
//! let container_name = "my-app";
//! let log_dir = Path::new("./logs");
//! let context_dir = Path::new("./app_context");
//!
//! // Use Docker utilities
//! let is_running = docker::state::container_running(container_name).await?;
//!
//! // Use Filesystem utilities
//! fs::io::ensure_dir_exists(log_dir)?;
//!
//! // Use Archive utilities
//! let tar_bytes = archive::tar::create_context_tar(context_dir)?;
//! # Ok(())
//! # }
//! ```
//!
//! This modular approach keeps the utility codebase organized and maintainable.
//!

/// Utilities for handling archive files (e.g., tarballs).
pub mod archive;
/// Core utilities for interacting with the Docker daemon (images, containers, etc.).
pub mod docker;
/// Utilities for filesystem operations (copying, I/O, links).
pub mod fs;
/// (Placeholder) Utilities related to network operations (IP detection, port scanning).
pub mod network;
/// (Placeholder) Utilities for executing and managing external processes.
pub mod process;
/// (Placeholder) Utilities for system-level information and checks (shell, tool detection).
pub mod system;
/// (Placeholder) Utilities for terminal user interface elements (progress, tables, prompts).
pub mod ui;
