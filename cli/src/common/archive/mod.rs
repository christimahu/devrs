//! # DevRS Archive Utilities Module (`common::archive`)
//!
//! File: cli/src/common/archive/mod.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module serves as the main interface and organizational unit for archive-related
//! utilities within the DevRS CLI. It aggregates functionality for creating, extracting,
//! and potentially manipulating archive files like tarballs. The primary initial use case
//! is creating tar archives for Docker build contexts.
//!
//! ## Architecture
//!
//! The module is designed to contain specialized submodules for different archive
//! formats or related operations:
//!
//! - **`tar`**: (Implemented) Provides functions specifically for creating TAR archives,
//!   particularly gzipped tarballs commonly used for Docker contexts.
//! - **`compression`**: (Placeholder) Intended to house utilities for various compression
//!   and decompression algorithms (e.g., gzip, bzip2, zstd) which might be used
//!   in conjunction with archiving or independently.
//!
//! ## Usage
//!
//! Functionality from submodules is typically accessed through this parent module, although
//! direct imports are possible.
//!
//! ```rust
//! use crate::common::archive; // Import the main archive module
//! use std::path::Path;
//!
//! // Example: Create a gzipped tar archive using the `tar` submodule's function
//! // (assuming `archive::tar::create_context_tar` is re-exported or accessed directly)
//! # async fn run() -> anyhow::Result<()> { // Example async context
//! let context_path = Path::new("./my_docker_context");
//! let tar_bytes = archive::tar::create_context_tar(context_path)?; // Using the tar submodule
//!
//! // The tar_bytes could then be used, e.g., sent to a Docker build endpoint
//! // ... docker::build_image(..., Some(tar_bytes.into())).await?;
//! # Ok(())
//! # }
//! ```
//!

pub mod tar;
// Placeholder for future compression utilities
// pub mod compression;
