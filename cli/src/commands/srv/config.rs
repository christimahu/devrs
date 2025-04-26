//! # DevRS HTTP Server Configuration
//!
//! File: cli/src/commands/srv/config.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module handles configuration loading, merging, and validation for
//! the HTTP file server functionality. It combines settings from:
//! 1. Command-line arguments (highest priority)
//! 2. Local configuration file `.devrs-srv.toml` (if present)
//! 3. Default values (lowest priority)
//!
//! ## Architecture
//!
//! The configuration system follows these steps:
//! 1. Parse command-line arguments
//! 2. Load configuration from file (if present)
//! 3. Merge settings (CLI args override file settings)
//! 4. Validate and resolve directory paths
//! 5. Create a unified ServerConfig structure
//!
//! ## Examples
//!
//! Configuration file format:
//!
//! ```toml
//! # Server configuration
//! port = 9000
//! host = "0.0.0.0"
//! directory = "public"
//! enable_cors = true
//! show_hidden = false
//! index_file = "index.html"
//! ```
//!
//! Loading and merging configuration:
//!
//! ```rust
//! // Parse CLI args
//! let args = SrvArgs::parse();
//!
//! // Load and merge config
//! let config = load_and_merge_config(args).await?;
//!
//! // Use the config
//! println!("Serving directory: {}", config.directory.display());
//! println!("Listening on: {}:{}", config.host, config.port);
//! ```
//!
//! The module ensures that all paths are resolved to absolute paths and
//! that the directory to serve actually exists and is accessible.
//!
use crate::core::error::Result;
use anyhow::Context;
use clap::Parser;
use serde::Deserialize;
use std::net::IpAddr;
use std::{env, fs, path::{Path, PathBuf}};
use tracing::{debug, info, warn};

/// The expected name for the server-specific configuration file.
const CONFIG_FILE_NAME: &str = ".devrs-srv.toml";

/// # Server Command Arguments (`SrvArgs`)
///
/// Defines the command-line arguments accepted by the `devrs srv` command,
/// parsed using `clap`. These arguments allow users to configure the server
/// directly from the command line, potentially overriding settings from a
/// configuration file or defaults.
#[derive(Parser, Debug)]
pub struct SrvArgs {
    /// Specifies the root directory from which files will be served.
    /// If not provided, defaults to the current working directory (`.`).
    #[arg(default_value = ".")]
    pub directory: PathBuf,

    /// Sets the network port the server will listen on.
    /// Defaults to port `8000`.
    #[arg(long, short, default_value_t = 8000)]
    pub port: u16,

    /// Sets the network IP address the server will bind to.
    /// Use `0.0.0.0` to bind to all available network interfaces, or `127.0.0.1`
    /// (the default) to only accept connections from the local machine.
    #[arg(long, default_value = "127.0.0.1")]
    pub host: IpAddr,

    /// Disables Cross-Origin Resource Sharing (CORS) headers.
    /// If this flag is present, CORS headers (like `Access-Control-Allow-Origin: *`)
    /// will *not* be sent. By default, CORS is enabled.
    #[arg(long)]
    pub no_cors: bool,

    /// Allows serving hidden files and directories (those starting with a dot `.`).
    /// By default, hidden files are not served or listed.
    #[arg(long)]
    pub show_hidden: bool,

    /// Specifies a custom name for the index file served when a directory is requested.
    /// Defaults to `"index.html"`.
    #[arg(long, short, default_value = "index.html")]
    pub index: String,
}

/// # Effective Server Configuration (`ServerConfig`)
///
/// Holds the final, consolidated configuration settings for the static file server
/// after merging command-line arguments and any settings loaded from a `.devrs-srv.toml`
/// configuration file. This struct contains the validated and resolved values that
/// the server logic will use.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)] // Disallow unknown fields during TOML deserialization
pub struct ServerConfig {
    /// The network port the server will listen on.
    pub port: u16,

    /// The network IP address the server will bind to.
    pub host: std::net::IpAddr,

    /// The resolved, absolute path to the directory being served.
    pub directory: PathBuf,

    /// Indicates whether CORS headers should be enabled.
    pub enable_cors: bool,

    /// Indicates whether hidden files (starting with `.`) should be served.
    pub show_hidden: bool,

    /// The name of the file to serve when a directory is requested (e.g., "index.html").
    pub index_file: String,
}

/// # Configuration from File (`FileConfig`)
///
/// A temporary helper struct used solely for deserializing the `.devrs-srv.toml` file.
/// All fields are optional (`Option<T>`) to allow users to specify only the settings
/// they wish to override from the defaults. This struct is then merged with defaults
/// and command-line arguments to produce the final `ServerConfig`.
#[derive(Deserialize, Debug)]
struct FileConfig {
    port: Option<u16>,
    host: Option<String>, // Read as string to handle potential parsing errors gracefully
    directory: Option<String>,
    enable_cors: Option<bool>,
    show_hidden: Option<bool>,
    index_file: Option<String>,
}

/// # Load and Merge Server Configuration (`load_and_merge_config`)
///
/// Orchestrates the process of determining the final server configuration.
/// It starts with default values, overrides them with settings found in a
/// `.devrs-srv.toml` file (if present in the target directory), and finally
/// overrides those with any explicitly provided command-line arguments.
/// The directory path is also resolved to an absolute path and validated.
///
/// ## Process:
/// 1. Create an initial `ServerConfig` based on command-line arguments (`args`).
/// 2. Determine the directory to search for the `.devrs-srv.toml` file (based on `args.directory`).
/// 3. Attempt to load settings from the configuration file using `load_config_from_dir`.
/// 4. If a configuration file is found and loaded successfully:
///    - Merge the file settings into the `effective_config`.
///    - Command-line arguments take precedence over file settings *only if* the arguments
///      differ from the program's default values (meaning the user explicitly set them).
///    - Boolean flags (`no_cors`, `show_hidden`) from the command line always override the file if used.
/// 5. If no configuration file is found, the `effective_config` remains based on `args`.
/// 6. Resolve the `effective_config.directory` to an absolute, canonical path and validate its existence.
///
/// ## Arguments
///
/// * `args`: The `SrvArgs` struct containing parsed command-line arguments.
///
/// ## Returns
///
/// * `Result<ServerConfig>`: A result containing the final, validated `ServerConfig` if successful,
///   or an error if configuration loading, merging, or validation fails.
///
/// ## Errors
///
/// Returns an error if:
/// - The current working directory cannot be determined.
/// - The configuration file exists but cannot be read or parsed (e.g., invalid TOML).
/// - The final resolved directory path does not exist, is not a directory, or cannot be accessed.
pub async fn load_and_merge_config(args: SrvArgs) -> Result<ServerConfig> {
    // Start with configuration directly derived from command-line arguments.
    let mut effective_config = ServerConfig::from_args(&args);
    let initial_dir_from_args = args.directory.clone();
    let cli_defaults = SrvArgs::parse_from([""]); // Get defaults for comparison

    // Resolve the potential location of the config file based on the input directory.
    let config_search_dir = if initial_dir_from_args.is_absolute() {
        initial_dir_from_args.clone()
    } else {
        // If relative, join with the current working directory.
        env::current_dir()
            .context("Failed to get current working directory")?
            .join(&initial_dir_from_args)
    };

    debug!(
        "Looking for config file in: {}",
        config_search_dir.display()
    );

    // Attempt to load configuration from the determined directory.
    if let Some(file_config) = load_config_from_dir(&config_search_dir)? {
        // Config file found, merge its settings. CLI args take precedence if explicitly set.
        info!(
            "Loaded settings from {}",
            config_search_dir.join(CONFIG_FILE_NAME).display()
        );

        // Port: Use file's value only if CLI arg was left at its default.
        if args.port == cli_defaults.port {
            effective_config.port = file_config.port;
        }
        // Host: Use file's value only if CLI arg was left at its default.
        if args.host == cli_defaults.host {
            effective_config.host = file_config.host;
        }
        // Index: Use file's value only if CLI arg was left at its default.
        if args.index == cli_defaults.index {
            effective_config.index_file = file_config.index_file;
        }
        // CORS: Use file's value only if --no-cors flag was *not* used.
        if !args.no_cors {
            effective_config.enable_cors = file_config.enable_cors;
        }
        // Hidden files: Use file's value only if --show-hidden flag was *not* used.
        if !args.show_hidden {
            effective_config.show_hidden = file_config.show_hidden;
        }

        // Directory: The directory loaded from the file is already resolved relative to the file.
        // This becomes the base directory if a config file was loaded.
        effective_config.directory = file_config.directory;

    } else {
        // No config file found or loaded. Use the directory specified in args.
        // `effective_config.directory` already holds `args.directory` from `from_args`.
        debug!("No config file found or loaded. Using arguments.");
    }

    // Final step: Resolve the potentially updated directory path and validate it.
    effective_config.resolve_directory().await?;

    Ok(effective_config)
}

/// # Default Server Configuration (`impl Default for ServerConfig`)
///
/// Provides the baseline default values for the `ServerConfig` struct.
/// These defaults are used if no configuration file is found and no overriding
/// command-line arguments are provided for a particular setting.
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 8000,
            host: std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), // Default to 127.0.0.1
            directory: PathBuf::from("."), // Default to current directory
            enable_cors: true,              // CORS enabled by default
            show_hidden: false,             // Hidden files disabled by default
            index_file: "index.html".to_string(), // Standard index file name
        }
    }
}

/// # Load Configuration from Directory (`load_config_from_dir`)
///
/// Attempts to find, read, and parse a `.devrs-srv.toml` configuration file
/// within the specified directory (`search_dir`).
///
/// If the file exists and is successfully parsed, it returns `Ok(Some(ServerConfig))`
/// containing the settings read from the file. Relative paths for the `directory`
/// setting within the file are resolved relative to the configuration file's location.
///
/// If the file does not exist, it returns `Ok(None)`.
///
/// ## Arguments
///
/// * `search_dir`: The directory (`PathBuf`) in which to look for the `.devrs-srv.toml` file.
///
/// ## Returns
///
/// * `Result<Option<ServerConfig>>`:
///     - `Ok(Some(config))` if the file was found and parsed successfully.
///     - `Ok(None)` if the file was not found.
///     - `Err(_)` if the file was found but could not be read or parsed (e.g., invalid TOML, permissions issue).
fn load_config_from_dir(search_dir: &Path) -> Result<Option<ServerConfig>> {
    // ... function body remains the same ...
    let config_path = search_dir.join(CONFIG_FILE_NAME);

    // Check if the configuration file exists and is actually a file.
    if !config_path.exists() || !config_path.is_file() {
        debug!("No config file found at {}", config_path.display());
        return Ok(None); // No config file found is not an error.
    }

    info!("Loading configuration from {}", config_path.display());

    // Read the file content.
    let content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

    // Parse the TOML content using the temporary FileConfig struct.
    let file_config: FileConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;

    // Get default values to fill in unspecified fields from the TOML file.
    let defaults = ServerConfig::default();

    // Parse the host IP address string, falling back to default if invalid.
    let host_ip = match file_config.host {
        Some(ref host_str) => host_str.parse().unwrap_or_else(|e| {
            warn!(
                "Invalid host IP '{}' in config file ({}), using default {}",
                host_str, e, defaults.host
            );
            defaults.host // Use default host if parsing fails.
        }),
        None => defaults.host, // Use default host if not specified in file.
    };

    // Determine the directory path from the config file, defaulting to "." if not specified.
    let directory_from_file = file_config.directory.as_deref().unwrap_or(".");
    let mut resolved_directory = PathBuf::from(directory_from_file);

    // If the directory path specified in the file is relative, resolve it
    // against the directory containing the config file itself.
    if resolved_directory.is_relative() {
        if let Some(config_parent_dir) = config_path.parent() {
            resolved_directory = config_parent_dir.join(&resolved_directory);
            debug!(
                "Resolved relative directory from config to: {}",
                resolved_directory.display()
            );
        } else {
            // This case is unlikely but possible if config_path is just a filename.
            warn!(
                "Could not get parent directory of config file {}, cannot resolve relative path '{}'",
                config_path.display(), directory_from_file
            );
            // Keep the potentially relative path; validation will handle it later.
        }
    }

    // Construct the ServerConfig from file values, using defaults where needed.
    Ok(Some(ServerConfig {
        port: file_config.port.unwrap_or(defaults.port),
        host: host_ip,
        directory: resolved_directory, // Use the resolved path.
        enable_cors: file_config.enable_cors.unwrap_or(defaults.enable_cors),
        show_hidden: file_config.show_hidden.unwrap_or(defaults.show_hidden),
        index_file: file_config.index_file.unwrap_or(defaults.index_file),
    }))
}

impl ServerConfig {
    /// # Create Configuration from Arguments (`from_args`)
    ///
    /// Creates an initial `ServerConfig` instance based *only* on the provided
    /// command-line arguments (`SrvArgs`). This serves as the starting point
    /// before potentially merging settings from a configuration file.
    /// Note that `enable_cors` is derived by inverting the `no_cors` argument flag.
    ///
    /// ## Arguments
    ///
    /// * `args`: The parsed command-line arguments (`SrvArgs`).
    ///
    /// ## Returns
    ///
    /// * `ServerConfig`: A configuration struct reflecting the command-line arguments.
    fn from_args(args: &SrvArgs) -> Self {
        Self {
            port: args.port,
            host: args.host,
            directory: args.directory.clone(), // Clone the PathBuf from args.
            enable_cors: !args.no_cors, // `enable_cors` is true if `no_cors` is false.
            show_hidden: args.show_hidden,
            index_file: args.index.clone(), // Clone the index file string.
        }
    }

    /// # Resolve and Validate Directory Path (`resolve_directory`)
    ///
    /// Ensures the `directory` field within this `ServerConfig` instance points to a valid,
    /// accessible, absolute directory path on the filesystem.
    ///
    /// ## Steps:
    /// 1. If the current `directory` path is relative, it's joined with the current working directory
    ///    to make it absolute.
    /// 2. The absolute path is then canonicalized using `tokio::fs::canonicalize` to resolve
    ///    any symbolic links and normalize the path representation (e.g., `.` and `..`).
    /// 3. Metadata for the canonical path is fetched using `tokio::fs::metadata`.
    /// 4. It verifies that the path exists and corresponds to a directory.
    /// 5. If all checks pass, the `directory` field of the `ServerConfig` is updated
    ///    with the canonicalized path.
    ///
    /// ## Returns
    ///
    /// * `Result<()>`: Returns `Ok(())` if the directory is successfully resolved and validated.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The current working directory cannot be obtained (needed for relative paths).
    /// - The path cannot be canonicalized (e.g., it doesn't exist, insufficient permissions).
    /// - Metadata cannot be retrieved for the canonical path.
    /// - The canonical path exists but does not point to a directory (e.g., it's a file).
    async fn resolve_directory(&mut self) -> Result<()> {
        let dir_path = &self.directory;

        // Ensure the path is absolute.
        let absolute_path = if dir_path.is_absolute() {
            dir_path.clone()
        } else {
            let current_dir =
                env::current_dir().context("Failed to get current working directory")?;
            current_dir.join(dir_path)
        };

        // Canonicalize the path asynchronously to resolve symlinks, ., .. etc.
        match tokio::fs::canonicalize(&absolute_path).await {
            Ok(canonical_path) => {
                // Path exists, now verify it's a directory.
                match tokio::fs::metadata(&canonical_path).await {
                    Ok(metadata) => {
                        if !metadata.is_dir() {
                            // Path exists but is not a directory.
                            anyhow::bail!("Path is not a directory: {}", canonical_path.display());
                        }
                        // Successfully validated, update the config's directory path.
                        self.directory = canonical_path;
                        debug!("Resolved serving directory to: {}", self.directory.display());
                    }
                    Err(e) => {
                        // Failed to get metadata even after canonicalization (permissions?).
                        anyhow::bail!(
                            "Failed to get metadata for path '{}': {}",
                            canonical_path.display(),
                            e
                        );
                    }
                }
            }
            Err(e) => {
                // Canonicalization failed, likely because the path doesn't exist or isn't accessible.
                anyhow::bail!(
                    "Directory '{}' could not be found or accessed: {}",
                    absolute_path.display(),
                    e
                );
            }
        }

        Ok(())
    }
}

// --- Unit Tests ---

/// # Unit Tests for Server Configuration
///
/// This module contains tests for the configuration loading, merging,
/// and validation logic within the `srv::config` module.
#[cfg(test)]
mod tests {
    use super::*; // Import items from the parent module (config.rs).
    use std::net::Ipv4Addr;
    use tempfile::TempDir; // Used for creating temporary directories for file-based tests.

    /// Test default configuration values.
    /// Verifies that `ServerConfig::default()` returns the expected baseline settings.
    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.port, 8000);
        assert_eq!(config.host, std::net::IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(config.directory, PathBuf::from("."));
        assert!(config.enable_cors); // Default is true
        assert!(!config.show_hidden); // Default is false
        assert_eq!(config.index_file, "index.html");
    }

    /// Test creating config solely from arguments.
    /// Verifies that `ServerConfig::from_args` correctly translates `SrvArgs`
    /// into a `ServerConfig`, including the inversion of the `no_cors` flag.
    #[test]
    fn test_from_args() {
        // Create SrvArgs with non-default values.
        let args = SrvArgs {
            directory: PathBuf::from("/test/dir"),
            port: 9000,
            host: "0.0.0.0".parse().unwrap(),
            no_cors: true,    // Should result in enable_cors: false
            show_hidden: true, // Should result in show_hidden: true
            index: "default.htm".to_string(),
        };

        let config = ServerConfig::from_args(&args);

        assert_eq!(config.port, 9000);
        assert_eq!(config.host, "0.0.0.0".parse::<std::net::IpAddr>().unwrap());
        assert_eq!(config.directory, PathBuf::from("/test/dir"));
        assert!(!config.enable_cors); // Check inversion
        assert!(config.show_hidden);
        assert_eq!(config.index_file, "default.htm");
    }

    /// Test loading configuration when no config file exists.
    /// Verifies that `load_config_from_dir` correctly returns `Ok(None)`
    /// when the `.devrs-srv.toml` file is not present in the search directory.
    #[tokio::test]
    async fn test_load_config_from_dir_no_file() -> Result<()> {
        let temp_dir = TempDir::new()?; // Create a temporary directory.
        let dir_path = temp_dir.path().to_path_buf();

        // Attempt to load config from the empty directory.
        let result = load_config_from_dir(&dir_path)?;
        assert!(result.is_none()); // Expect None as no file exists.

        Ok(())
    }

    /// Test loading and parsing a valid configuration file.
    /// Verifies that `load_config_from_dir` reads, parses, and correctly interprets
    /// settings from a `.devrs-srv.toml` file, including resolving relative directory paths.
    #[tokio::test]
    async fn test_load_config_from_dir_with_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let dir_path = temp_dir.path().to_path_buf();

        // Define config file content with various settings.
        let config_content = r#"
        port = 9090
        host = "0.0.0.0"
        directory = "public" # Relative path
        enable_cors = false
        show_hidden = true
        index_file = "home.html"
        "#;

        // Write the content to the config file within the temp directory.
        fs::write(dir_path.join(CONFIG_FILE_NAME), config_content)?;

        // Attempt to load the config.
        let result = load_config_from_dir(&dir_path)?;
        assert!(result.is_some()); // Expect Some(config).

        let config = result.unwrap();
        // Verify the values loaded from the file.
        assert_eq!(config.port, 9090);
        assert_eq!(config.host, "0.0.0.0".parse::<std::net::IpAddr>().unwrap());
        // Directory should be resolved relative to the config file's location (the temp dir).
        assert_eq!(config.directory, dir_path.join("public"));
        assert!(!config.enable_cors);
        assert!(config.show_hidden);
        assert_eq!(config.index_file, "home.html");

        Ok(())
    }

    /// Test directory resolution for an existing directory.
    /// Verifies that `resolve_directory` correctly canonicalizes an existing path.
    #[tokio::test]
    async fn test_resolve_directory_existing() -> Result<()> {
        let temp_dir = TempDir::new()?; // Use a known existing directory.
        let dir_path = temp_dir.path().to_path_buf();

        let mut config = ServerConfig {
            directory: dir_path.clone(), // Start with the temp directory path.
            ..ServerConfig::default()    // Use defaults for other fields.
        };

        // Resolve the directory.
        config.resolve_directory().await?;

        // Check that the directory path in the config is now the canonicalized path.
        assert_eq!(config.directory, fs::canonicalize(dir_path)?);

        Ok(())
    }

    /// Test directory resolution for a non-existent directory.
    /// Verifies that `resolve_directory` returns an error when the path does not exist.
    #[tokio::test]
    async fn test_resolve_directory_nonexistent() {
        let mut config = ServerConfig {
            directory: PathBuf::from("/path/that/definitely/does/not/exist"),
            ..ServerConfig::default()
        };

        // Attempt to resolve the non-existent directory.
        let result = config.resolve_directory().await;
        assert!(result.is_err()); // Expect an error.
    }

    /// Test the complete load and merge process with args and file.
    /// Verifies the precedence rules: explicitly set CLI args override file settings,
    /// which in turn override defaults. Also checks boolean flag handling.
    #[tokio::test]
    async fn test_load_and_merge_config() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let dir_path = temp_dir.path().to_path_buf();
        let serve_subdir = temp_dir.path().join("www");
        fs::create_dir(&serve_subdir)?; // Create the actual directory to be served

        // Create a config file with some settings.
        let config_content = r#"
        port = 9090         # Non-default port
        host = "0.0.0.0"    # Non-default host
        directory = "www"   # Relative directory within temp_dir
        enable_cors = false # Non-default CORS
        show_hidden = true  # Non-default hidden files
        index_file = "home.html" # Non-default index
        "#;
        fs::write(dir_path.join(CONFIG_FILE_NAME), config_content)?;

        // Simulate CLI args: Some explicit, some left at default.
        let args = SrvArgs {
            directory: dir_path.clone(),      // Explicitly point to dir containing the config file
            port: 8000,                       // CLI uses default port
            host: Ipv4Addr::LOCALHOST.into(), // CLI uses default host
            no_cors: true,                    // Explicitly set --no-cors (overrides file)
            show_hidden: false,               // CLI uses default hidden file setting
            index: "index.html".into(),       // CLI uses default index file
        };

        // Perform the load and merge operation.
        let config = load_and_merge_config(args).await?;

        // --- Assertions ---
        // Port: CLI was default (8000), file is 9090. Expect file value.
        assert_eq!(config.port, 9090);
        // Host: CLI was default (127.0.0.1), file is 0.0.0.0. Expect file value.
        assert_eq!(config.host.to_string(), "0.0.0.0");
        // CORS: CLI explicitly set --no-cors. Expect CLI value (false).
        assert!(!config.enable_cors);
        // Hidden: CLI was default (false), file is true. Expect file value.
        assert!(config.show_hidden);
        // Index: CLI was default ("index.html"), file is "home.html". Expect file value.
        assert_eq!(config.index_file, "home.html");
        // Directory: Config file specified "www" relative to dir_path. Expect resolved absolute path.
        assert_eq!(config.directory, fs::canonicalize(&serve_subdir)?);

        Ok(())
    }

    /// Test merging when CLI overrides non-default file values.
    #[tokio::test]
    async fn test_load_and_merge_cli_overrides_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let dir_path = temp_dir.path().to_path_buf();
        let serve_subdir = temp_dir.path().join("data");
        fs::create_dir(&serve_subdir)?;

        // Config file with non-default settings
        let config_content = r#"
        port = 9001
        host = "192.168.1.1"
        directory = "data"
        enable_cors = true
        show_hidden = false
        index_file = "default.php"
        "#;
        fs::write(dir_path.join(CONFIG_FILE_NAME), config_content)?;

        // CLI args explicitly overriding *all* file settings
        let args = SrvArgs {
            directory: dir_path.clone(), // Point to dir containing config
            port: 9999,                  // Explicit CLI port
            host: "10.0.0.1".parse()?,   // Explicit CLI host
            no_cors: true,               // Explicit CLI CORS setting (disable)
            show_hidden: true,           // Explicit CLI hidden setting (enable)
            index: "app.htm".into(),     // Explicit CLI index
        };

        let config = load_and_merge_config(args).await?;

        // Assert that all CLI values took precedence
        assert_eq!(config.port, 9999);
        assert_eq!(config.host.to_string(), "10.0.0.1");
        assert!(!config.enable_cors); // from --no-cors
        assert!(config.show_hidden);
        assert_eq!(config.index_file, "app.htm");
        // Directory comes from the file config if present, but then gets resolved.
        assert_eq!(config.directory, fs::canonicalize(&serve_subdir)?);

        Ok(())
    }

    /// Test merging when only args are provided (no config file).
    #[tokio::test]
    async fn test_load_and_merge_args_only() -> Result<()> {
        let temp_dir = TempDir::new()?; // Use a temporary directory
        let dir_path = temp_dir.path().to_path_buf(); // This will be the served directory

        // CLI args with specific settings
        let args = SrvArgs {
            directory: dir_path.clone(), // Serve the temp dir
            port: 8080,
            host: "0.0.0.0".parse()?,
            no_cors: false, // Explicitly *don't* disable CORS
            show_hidden: true,
            index: "main.js".into(),
        };

        // No config file exists in temp_dir

        let config = load_and_merge_config(args).await?;

        // Assert that the config reflects the arguments directly
        assert_eq!(config.port, 8080);
        assert_eq!(config.host.to_string(), "0.0.0.0");
        assert!(config.enable_cors); // Since --no-cors was false
        assert!(config.show_hidden);
        assert_eq!(config.index_file, "main.js");
        assert_eq!(config.directory, fs::canonicalize(&dir_path)?); // Resolved path

        Ok(())
    }
}
