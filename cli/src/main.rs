//! # DevRS Main Entry Point
//!
//! File: cli/src/main.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten 
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This file serves as the main entry point for the DevRS CLI application.
//! It handles:
//! - Command-line argument parsing using Clap
//! - Setting up the logging system based on verbosity flags
//! - Routing execution to appropriate command handlers
//!
//! ## Architecture
//!
//! The application follows a modular command structure:
//! - Each top-level command (`env`, `container`, etc.) is defined as a variant in the `Commands` enum
//! - Commands are mapped to handler functions in their respective modules
//! - All errors are propagated to this level for consistent handling
//!
//! ## Examples
//!
//! Basic DevRS usage:
//!
//! ```bash
//! # Get help
//! devrs --help
//!
//! # Run a command with increased verbosity
//! devrs -vv env build
//! ```
//!
//! Command processing flow:
//! 1. Parse command-line args via Clap
//! 2. Configure logging based on verbosity level
//! 3. Route to appropriate command handler
//! 4. Format and display any errors that occur
//!
use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

// Declare the top-level modules of the CLI crate.
mod commands; // Handles specific command logic (env, container, etc.)
mod common; // Contains shared utilities (docker, fs, etc.)
mod core; // Core infrastructure (errors, config, templating, etc.)

// Note: Removed `mod fsutils;` - functionality moved to common/fs/*
// Note: error, config, templating are now declared within core/mod.rs

/// Defines the top-level command-line arguments structure using Clap's derive macros.
#[derive(Parser, Debug)]
#[command(
    name = "devrs",
    about = "🦀 DevRS ⚙️: Containerized Development Environment & Tooling",
    long_about = "Manage core dev environment, application containers, blueprints, and setup.\n\
                  Provides a unified CLI for consistent development workflows.",
    propagate_version = true,
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,
}

/// Enum defining all available top-level commands.
#[derive(Parser, Debug)]
enum Commands {
    #[command(alias = "e")]
    Env(commands::env::EnvArgs),
    #[command(alias = "c")]
    Container(commands::container::ContainerArgs),
    #[command(alias = "b")]
    Blueprint(commands::blueprint::BlueprintArgs),
    #[command(alias = "s")]
    Setup(commands::setup::SetupArgs),
    Srv(commands::srv::SrvArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Use anyhow::Result directly
    let cli = Cli::parse();

    let log_level = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));
    fmt::Subscriber::builder()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .compact()
        .init();

    tracing::debug!("Parsed CLI arguments: {:?}", cli);

    let command_result = match cli.command {
        Commands::Env(args) => commands::env::handle_env(args).await,
        Commands::Container(args) => commands::container::handle_container(args).await,
        Commands::Blueprint(args) => commands::blueprint::handle_blueprint(args).await,
        Commands::Setup(args) => commands::setup::handle_setup(args).await,
        Commands::Srv(args) => commands::srv::handle_srv(args).await,
    };

    if let Err(e) = command_result {
        tracing::error!("Command execution failed: {:?}", e);
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

// --- Basic Integration Tests --- (Remain the same)
#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use predicates::prelude::*;
    fn devrs_cmd() -> Command {
        Command::cargo_bin("devrs").expect("Failed to find devrs binary for testing")
    }
    #[test]
    fn test_main_help_flag() {
        devrs_cmd().arg("--help").assert().success();
    }
    #[test]
    fn test_main_version_flag() {
        devrs_cmd()
            .arg("--version")
            .assert()
            .success()
            .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
    }
}
