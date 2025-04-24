//! # DevRS Filesystem Utilities (`common::fs`)
//!
//! File: cli/src/common/fs/mod.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!

//! ## Overview
//!
//! This module acts as the primary interface and organizational unit for all
//! filesystem-related utility functions within the DevRS CLI. It aggregates
//! functionality from specialized submodules, providing a consistent entry point
//! for operations like file I/O, directory manipulation, copying, and symbolic
//! link management.
//!
//! ## Architecture
//!
//! Functionality is delegated to the following submodules:
//!
//! - **`copy`**: Handles recursive copying of directories, currently using the `fs_extra` crate. Primarily used by `devrs blueprint create`.
//! - **`io`**: Provides basic input/output operations like ensuring directories exist (`ensure_dir_exists`), reading files to strings (`read_file_to_string`), and writing strings to files (`write_string_to_file`). Used widely across commands.
//! - **`links`**: Manages the creation and validation of symbolic links, including platform-specific handling and backup of existing targets. Used by `devrs setup`.
//!
//! While key functions *could* be re-exported here for convenience (using `pub use`), the current structure requires importing from the specific submodule (e.g., `crate::common::fs::io::ensure_dir_exists`).
//!
//! ## Usage
//!
//! Other parts of the application typically import the specific submodule needed.
//!
//! ```rust
//! use crate::common::fs::{copy, io, links}; // Import desired submodules
//! use crate::core::error::Result;
//! use std::path::Path;
//!
//! # fn run_example() -> Result<()> {
//! let source_path = Path::new("./src");
//! let target_path = Path::new("./dest");
//! let file_path = Path::new("./my_file.txt");
//! let link_target = Path::new("./link_to_file");
//! let content = "File content.";
//!
//! // Use functions from the 'io' submodule
//! io::ensure_dir_exists(target_path)?;
//! io::write_string_to_file(file_path, content)?;
//! let read_content = io::read_file_to_string(file_path)?;
//! assert_eq!(content, read_content);
//!
//! // Use functions from the 'copy' submodule
//! copy::copy_directory_recursive(source_path, target_path)?;
//!
//! // Use functions from the 'links' submodule
//! links::create_symlink(file_path, link_target)?;
//! # Ok(())
//! # }
//! ```
//!

/// Contains functions for copying files and directories (e.g., `copy_directory_recursive`).
pub mod copy;
/// Contains basic file I/O operations (e.g., `ensure_dir_exists`, `read_file_to_string`, `write_string_to_file`).
pub mod io;
/// Contains functions for managing symbolic links (e.g., `create_symlink`).
pub mod links;

// Note: No re-exports are currently defined here. Users need to import from submodules,
// e.g., `use crate::common::fs::io::ensure_dir_exists;`
// Consider adding re-exports if preferred:
// `pub use io::{ensure_dir_exists, read_file_to_string, write_string_to_file};`
// `pub use copy::copy_directory_recursive;`
// `pub use links::create_symlink;`
