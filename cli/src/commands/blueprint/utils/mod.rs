//! # DevRS Blueprint Utilities
//!
//! File: cli/src/commands/blueprint/utils/mod.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten 
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module aggregates utility functions specifically designed to support
//! the `devrs blueprint` command group (list, info, create). By organizing
//! these helpers here, we avoid code duplication and promote maintainability.
//!
//! ## Architecture
//!
//! The module is organized into specialized submodules:
//!
//! - `project_detector`: Functions to analyze a blueprint's contents and
//!   determine the likely project type (e.g., Rust, Go) and build system.
//! - `tree_printer`: Functionality to display a directory's structure
//!   in a formatted, tree-like view, useful for the `blueprint info` command.
//!
//! ## Usage
//!
//! These utilities are primarily intended for internal use by the blueprint
//! command handlers, but can be imported by other modules if needed:
//!
//! ```rust
//! use crate::commands::blueprint::utils::project_detector;
//! use crate::commands::blueprint::utils::tree_printer;
//! 
//! // Detect project type from a directory
//! let project_info = project_detector::detect_project_type(&path);
//! 
//! // Print directory structure
//! let tree_string = tree_printer::print_directory_tree_to_string(&path, "my_project")?;
//! ```
//!

// --- Submodules for Blueprint Utilities ---

/// # Project Detector (`project_detector`)
///
/// This submodule contains logic to inspect the files within a blueprint
/// directory (e.g., looking for `Cargo.toml`, `package.json`, `CMakeLists.txt`)
/// to heuristically determine the primary programming language or framework
/// and the associated build system or package manager. This information is
/// primarily used by the `devrs blueprint info` command to display details
/// about a template.
pub mod project_detector;

/// # Tree Printer (`tree_printer`)
///
/// This submodule provides functionality to generate a visual, tree-like
/// representation of a directory's structure, similar to the output of the
/// standard `tree` command-line utility. It includes indentation and connection
/// lines (`├──`, `└──`) and is used by the `devrs blueprint info` command
/// to show the file layout of a blueprint template.
pub mod tree_printer;
