//! # DevRS Core Infrastructure
//!
//! File: cli/src/core/mod.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten 
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module aggregates the core infrastructure components that provide
//! foundational functionality for the DevRS application. These components
//! handle configuration, error management, and templating.
//!
//! ## Architecture
//!
//! The core infrastructure consists of three key components:
//! - `config`: Configuration loading, merging, and validation
//! - `error`: Error types and error handling utilities
//! - `templating`: Template rendering for project blueprints
//!
//! These components provide essential infrastructure that's used by
//! the command modules to implement their functionality.
//!
//! ## Usage
//!
//! Core infrastructure is imported by command handlers:
//!
//! ```rust
//! use crate::core::config; // For loading configuration
//! use crate::core::error::{DevrsError, Result}; // For error handling
//! use crate::core::templating; // For blueprint template rendering
//! ```
//!
//! These modules provide foundational capabilities that are used across
//! different parts of the application, ensuring consistent behavior.
//!
pub mod config;
pub mod error;
pub mod templating;
