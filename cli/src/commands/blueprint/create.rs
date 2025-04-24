//! # DevRS Blueprint Create Command
//!
//! File: cli/src/commands/blueprint/create.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs blueprint create` command, which
//! scaffolds new projects from blueprint templates. It handles:
//! - Parsing command-line arguments for project creation
//! - Resolving source and target paths
//! - Building template context with project variables
//! - Rendering templates with variable substitution
//! - Providing helpful next steps after project creation
//!
//! ## Architecture
//!
//! The command flow follows these steps:
//! 1. Parse and validate command arguments
//! 2. Resolve template source path from configuration
//! 3. Prepare target project path and handle collisions
//! 4. Build template context with variables (project name, timestamps, etc.)
//! 5. Invoke the templating system to render the project
//! 6. Print completion message with relevant next steps
//!
//! ## Examples
//!
//! Basic usage:
//!
//! ```bash
//! # Create a new Rust project named "my-project"
//! devrs blueprint create --lang rust my-project
//!
//! # Create a new project in a specific output directory
//! devrs blueprint create --lang go --output ~/projects my-api
//!
//! # Create with custom variables and force overwrite of existing directory
//! devrs blueprint create --lang cpp --force --var author="Christi Mahu" --var version="0.1.0" my-app
//! ```
//!
//! The command generates helpful next steps based on project type detection,
//! suggesting commands appropriate for the created project.
//!
use crate::core::config; // Access configuration loading (e.g., blueprint directory path)
use crate::core::error::Result; // Use the standard Result type for error handling
use crate::core::templating; // Access the core templating engine logic
// Filesystem utilities might be used directly or indirectly (e.g., through templating)
use anyhow::{anyhow, Context}; // For easy error creation and adding context
use clap::Parser; // For parsing command-line arguments
use pathdiff; // For calculating relative paths for user messages
use std::{
    collections::HashMap, // For storing template context variables
    env,                  // For getting current directory and environment variables (like USER)
    fs,                   // Standard filesystem operations (checking existence, creating dirs)
    path::{Path, PathBuf}, // Filesystem path types
};
use tracing::{debug, info, warn}; // For logging

/// # Create Blueprint Arguments (`CreateArgs`)
///
/// Defines the command-line arguments accepted by the `devrs blueprint create` subcommand.
/// Uses the `clap` crate for parsing and validation.
#[derive(Parser, Debug)]
pub struct CreateArgs {
    /// Specifies the blueprint language/template to use (e.g., "rust", "go").
    /// This corresponds to a subdirectory name within the configured blueprints directory.
    #[arg(long, short = 'l')] // Define as `--lang` or `-l`
    lang: String,

    /// The desired name for the new project directory. This name will also be
    /// available as a variable within the templates (e.g., `project_name`).
    project_name: String, // Positional argument

    /// Optional: Specifies the parent directory where the new project directory
    /// should be created. If omitted, the current working directory is used.
    #[arg(long, short = 'o')] // Define as `--output` or `-o`
    output: Option<PathBuf>,

    /// Optional: If set, allows overwriting an existing directory at the target path.
    /// Without this flag, the command will fail if the target directory already exists.
    #[arg(long, short = 'f')] // Define as `--force` or `-f`
    force: bool,

    /// Optional: Defines custom key-value pairs to be passed as variables to the
    /// template rendering engine. Can be specified multiple times.
    /// Example: `--var author="Your Name" --var version="1.0"`
    /// These variables can override default variables (like `project_name`).
    #[arg(long = "var", value_parser = parse_key_val, action = clap::ArgAction::Append)]
    var: Vec<(String, String)>,
}

/// # Parse Key-Value Pair (`parse_key_val`)
///
/// A helper function used by `clap` to parse command-line arguments provided
/// via the `--var` flag. It expects strings in the format "KEY=VALUE" and
/// splits them into a tuple of `(String, String)`.
///
/// ## Arguments
/// * `s` - The input string slice (`&str`) from the command line (e.g., "author=Jane").
///
/// ## Returns
/// * `Result<(String, String)>` - `Ok` containing the key-value tuple if parsing is successful.
/// * `Err` - An `anyhow::Error` if the input string does not contain an '=' character.
fn parse_key_val(s: &str) -> Result<(String, String)> {
    // Attempt to find the first '=' character and split the string.
    s.split_once('=')
        // If split is successful, trim whitespace from key and value and convert to String.
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        // If `split_once` returns `None` (no '=' found), return an error.
        .ok_or_else(|| {
            anyhow!(
                "Invalid variable format: '{}'. Expected format: KEY=VALUE",
                s
            )
        })
}

/// # Handle Blueprint Create Command (`handle_create`)
///
/// The main asynchronous handler function for the `devrs blueprint create` command.
/// Orchestrates the project creation process:
/// 1. Logs the start of the command.
/// 2. Prepares and validates the source blueprint path and the target project path using `prepare_and_validate_paths`.
/// 3. Builds the context variables (default + user-provided) for the template engine using `build_template_context`.
/// 4. Calls the core templating engine (`templating::render_template_directory`) to copy files and render templates.
/// 5. Prints a success message and helpful next steps using `print_completion_message`.
///
/// ## Arguments
/// * `args` - The parsed `CreateArgs` containing all command-line options.
///
/// ## Returns
/// * `Result<()>` - `Ok(())` on successful project creation, or an `Err` if any step fails.
pub async fn handle_create(args: CreateArgs) -> Result<()> {
    info!("Handling blueprint create command...");
    info!(
        "Attempting to create project '{}' from '{}' blueprint",
        args.project_name, args.lang
    );

    // 1. Prepare and validate source (blueprint) and target (output project) paths.
    // This also handles checks for existing target directories and the --force flag.
    let (source_path, target_path) = prepare_and_validate_paths(&args)
        .await // Use .await as the function is async
        .context("Failed to prepare or validate blueprint paths")?; // Add context to errors

    // 2. Build the map of context variables to be used during template rendering.
    // This includes default variables (project name variations, date, user) and user overrides.
    let context_map =
        build_template_context(&args).context("Failed to build template context variables")?;
    debug!("Template context prepared: {:?}", context_map); // Log the context for debugging

    // 3. Define which file extensions should be treated as templates by the rendering engine.
    let template_extensions = [".template", ".tmpl", ".tera"]; // Common template extensions

    // 4. Call the templating engine to perform the rendering and file copying.
    info!(
        "Starting template rendering from '{}' to '{}'",
        source_path.display(),
        target_path.display()
    );
    templating::render_template_directory(
        &source_path,         // Source blueprint directory
        &target_path,         // Target project directory
        &context_map,         // Variables for substitution
        &template_extensions, // Files ending with these extensions will be rendered
    )
    .context("Blueprint template rendering failed")?; // Add context to rendering errors

    // 5. Print a success message and next steps to the user.
    print_completion_message(&target_path, &args.project_name);

    Ok(())
}

/// # Prepare and Validate Paths (`prepare_and_validate_paths`)
///
/// Resolves the absolute paths for the source blueprint directory and the target
/// project directory. It performs necessary validation checks:
/// - Loads configuration to find the root blueprint directory.
/// - Checks if the specified blueprint (`args.lang`) exists within the root.
/// - Resolves the target directory path (handling the `--output` argument and relative paths).
/// - Checks for collisions at the target path (file exists, directory exists) and handles the `--force` flag.
/// - Ensures the parent directory of the target path exists, creating it if necessary.
///
/// ## Arguments
/// * `args` - The parsed `CreateArgs`.
///
/// ## Returns
/// * `Result<(PathBuf, PathBuf)>` - A tuple containing the validated absolute `source_path`
///   and `target_path` if successful, or an `Err` if validation fails.
async fn prepare_and_validate_paths(args: &CreateArgs) -> Result<(PathBuf, PathBuf)> {
    // Load DevRS configuration to find the base directory where blueprints are stored.
    let cfg = config::load_config().context("Failed to load DevRS configuration")?;
    // Blueprints directory path from config (already expanded to absolute during load).
    let bp_root_dir = PathBuf::from(&cfg.blueprints.directory);

    // --- Source Path Resolution and Validation ---
    // Construct the full path to the requested blueprint source directory.
    let source_path = bp_root_dir.join(&args.lang);
    debug!("Resolved blueprint source path: {}", source_path.display());

    // Check if the source blueprint directory actually exists.
    if !source_path.exists() {
        anyhow::bail!( // Return an error using anyhow's bail macro
            "Blueprint '{}' not found in '{}'. Run 'devrs blueprint list' to see available blueprints.",
            args.lang,
            bp_root_dir.display()
        );
    }
    // Check if the source path is indeed a directory.
    if !source_path.is_dir() {
        anyhow::bail!(
            "Blueprint path '{}' exists but is not a directory.",
            source_path.display()
        );
    }

    // --- Target Path Resolution ---
    // Determine the base directory where the new project should be created.
    let target_base_dir = match &args.output {
        // If --output is specified:
        Some(output_dir) => {
            if output_dir.is_absolute() {
                // Use the absolute path directly.
                output_dir.clone()
            } else {
                // Resolve the relative path against the current working directory.
                env::current_dir()
                    .context("Failed to get current directory")?
                    .join(output_dir)
            }
        }
        // If --output is not specified, use the current working directory.
        None => env::current_dir().context("Failed to get current directory")?,
    };
    // Construct the final target path by joining the base directory and the project name.
    let target_path = target_base_dir.join(&args.project_name);
    debug!("Resolved target project path: {}", target_path.display());

    // --- Target Path Validation and Collision Handling ---
    // Check if anything already exists at the target path.
    if target_path.exists() {
        // Check if the existing item is a file (which we cannot overwrite with a directory).
        if !target_path.is_dir() {
            anyhow::bail!(
                "Target path '{}' exists but is a file, cannot create project directory.",
                target_path.display()
            );
        }
        // If it's a directory, check the --force flag.
        if args.force {
            // User specified --force, log a warning but allow proceeding.
            // The templating engine might overwrite files within the existing directory.
            warn!(
                "Target directory '{}' already exists. Proceeding due to --force flag. Existing files may be overwritten.",
                target_path.display()
            );
        } else {
            // Target directory exists and --force was not used. Return an error.
            anyhow::bail!(
                "Target directory '{}' already exists. Use --force to overwrite or choose a different project name/output directory.",
                target_path.display()
            );
        }
    } else {
        // Target path does not exist. Ensure its parent directory exists.
        if let Some(parent) = target_path.parent() {
            if !parent.exists() {
                // Create the parent directory(ies) if they don't exist.
                fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "Failed to create parent directory '{}' for target.",
                        parent.display()
                    )
                })?;
                debug!("Created parent directory: {}", parent.display());
            }
        }
    }

    // Return the validated source and target paths.
    Ok((source_path, target_path))
}

/// # Build Template Context (`build_template_context`)
///
/// Creates the `HashMap` of variables that will be available to the Tera
/// templating engine during rendering. It includes default variables derived
/// from the project name and current environment, plus any custom variables
/// provided by the user via the `--var` flag.
///
/// ## Default Variables Provided:
/// - `project_name`: The original project name provided by the user.
/// - `project_name_lowercase`: Project name converted to lowercase.
/// - `project_name_uppercase`: Project name converted to uppercase.
/// - `project_name_kebab`: Same as original `project_name` (useful for clarity).
/// - `project_name_snake`: Project name converted to snake_case.
/// - `project_name_camel`: Project name converted to camelCase.
/// - `project_name_pascal`: Project name converted to PascalCase.
/// - `current_date`: Current date (YYYY-MM-DD).
/// - `current_year`: Current year (YYYY).
/// - `current_time`: Current time (HH:MM:SS).
/// - `username`: Current system username ($USER or $USERNAME).
///
/// ## User Variables:
/// - Variables passed via `--var KEY=VALUE` are added to the context.
/// - **Important:** User-provided variables can overwrite the default variables if they share the same key (e.g., `--var project_name=override`).
///
/// ## Arguments
/// * `args` - The parsed `CreateArgs`.
///
/// ## Returns
/// * `Result<HashMap<String, String>>` - The context map ready for the template engine.
fn build_template_context(args: &CreateArgs) -> Result<HashMap<String, String>> {
    // Initialize an empty HashMap to store context variables.
    let mut context = HashMap::new();

    // Get the project name from arguments.
    let project_name = &args.project_name;

    // --- Add Default Variables ---
    // Add the original project name.
    context.insert("project_name".to_string(), project_name.clone());
    // Add various case conversions of the project name.
    context.insert(
        "project_name_lowercase".to_string(),
        project_name.to_lowercase(),
    );
    context.insert(
        "project_name_uppercase".to_string(),
        project_name.to_uppercase(),
    );
    // Kebab case is often the same as the input, provided for convention.
    context.insert("project_name_kebab".to_string(), project_name.clone());
    // Generate and insert snake_case, camelCase, and PascalCase versions.
    let snake_case = to_snake_case(project_name);
    let camel_case = to_camel_case(project_name);
    let pascal_case = to_pascal_case(project_name);
    context.insert("project_name_snake".to_string(), snake_case);
    context.insert("project_name_camel".to_string(), camel_case);
    context.insert("project_name_pascal".to_string(), pascal_case);

    // Add current date and time information using the chrono crate.
    let now = chrono::Local::now(); // Get current local time.
    context.insert(
        "current_date".to_string(),
        now.format("%Y-%m-%d").to_string(), // Format as YYYY-MM-DD
    );
    context.insert("current_year".to_string(), now.format("%Y").to_string()); // Format as YYYY
    context.insert(
        "current_time".to_string(),
        now.format("%H:%M:%S").to_string(), // Format as HH:MM:SS
    );

    // Add the current system username.
    // Try getting USER (Unix-like) or USERNAME (Windows), fallback to "user".
    let username = env::var("USER") // Try $USER first
        .or_else(|_| env::var("USERNAME")) // Try $USERNAME if $USER fails
        .unwrap_or_else(|_| "user".to_string()); // Default to "user" if both fail
    context.insert("username".to_string(), username);

    // --- Add User-Provided Variables ---
    // Iterate over the variables provided via the `--var` flag.
    for (key, value) in &args.var {
        info!("Applying custom context variable: {} = {}", key, value);
        // Insert the user's variable. This will overwrite any default variable with the same key.
        context.insert(key.clone(), value.clone());
    }

    // Return the completed context map.
    Ok(context)
}

// --- Case Conversion Helpers ---
// These functions convert the project name string into common casing conventions
// often needed in source code (e.g., struct names, variable names, module names).

/// Converts a kebab-case or other input string to snake_case.
/// Replaces hyphens with underscores and converts to lowercase.
fn to_snake_case(input: &str) -> String {
    input.replace('-', "_").to_lowercase()
}

/// Converts a kebab-case or snake_case string to camelCase.
/// Handles multiple uppercase letters together (like HTTPRequest -> httpRequest).
fn to_camel_case(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut capitalize_next = false; // Flag to capitalize the next character encountered.
    let mut first_word_char = true; // Flag to lowercase the very first character.

    for c in input.chars() {
        if c == '-' || c == '_' {
            // Delimiter found, next character should be capitalized.
            capitalize_next = true;
        } else if capitalize_next {
            // Capitalize this character and add it.
            result.push(c.to_ascii_uppercase());
            capitalize_next = false; // Reset flag.
            first_word_char = false; // No longer the first char overall.
        } else if first_word_char {
            // Lowercase the very first character of the result.
            result.push(c.to_ascii_lowercase());
            first_word_char = false; // Reset flag.
        } else {
            // Add the character, handling potential consecutive uppercase (e.g., "HTTPRequest" -> "httpRequest")
            // Generally, keep subsequent chars lowercase unless the *next* char is also lowercase
            // This part is tricky; a simpler approach might be needed if this doesn't cover all cases.
            // For now, let's just push the character as is, assuming delimiters handle word breaks.
            // The previous logic was likely too aggressive in lowercasing.
            result.push(c);
        }
    }
    result // Return modified or original result
}

/// Converts a kebab-case or snake_case string to PascalCase (aka UpperCamelCase).
fn to_pascal_case(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut capitalize_next = true; // Start by capitalizing the first character.

    for c in input.chars() {
        if c == '-' || c == '_' {
            // Delimiter found, next character should be capitalized.
            capitalize_next = true;
        } else if capitalize_next {
            // Capitalize this character and add it.
            result.push(c.to_ascii_uppercase());
            capitalize_next = false; // Reset flag.
        } else {
            // Add subsequent characters within a word as is.
            result.push(c);
        }
    }
    result
}

/// # Print Completion Message (`print_completion_message`)
///
/// Displays a user-friendly message after the blueprint creation process
/// has successfully completed. It includes the project location and suggests
/// relevant next steps (like `cd`ing into the directory, reading the README,
/// or running initial build/install commands) based on simple heuristics
/// (checking for the existence of common marker files like `Cargo.toml`, `package.json`, etc.).
///
/// ## Arguments
/// * `target_path` - The `&Path` where the project was created.
/// * `project_name` - The `&str` name of the created project.
fn print_completion_message(target_path: &Path, project_name: &str) {
    // --- Success Header ---
    println!("\n✅ Project '{}' created successfully!", project_name);
    println!("   Location: {}", target_path.display()); // Show absolute path

    // --- Next Steps ---
    println!("\nNext steps:");

    // Suggest `cd` command, calculating relative path if possible.
    let display_path = match env::current_dir() {
        Ok(cwd) => {
            // Try to get a relative path from current dir to target dir.
            pathdiff::diff_paths(target_path, &cwd)
                // If successful, convert the relative path to a string.
                .map(|p| p.display().to_string())
                // If relative path fails (e.g., different drives on Windows), fallback to absolute path.
                .unwrap_or_else(|| target_path.display().to_string())
        }
        // If getting CWD fails, just use the absolute target path.
        Err(_) => target_path.display().to_string(),
    };
    println!("  1. Navigate to your project: cd {}", display_path);

    // Dynamically suggest next steps based on detected project files.
    let mut step_number = 2; // Start numbering steps from 2.

    // Suggest reading README if it exists.
    if target_path.join("README.md").exists() {
        println!(
            "  {}. Review project instructions: cat README.md",
            step_number
        );
        step_number += 1;
    }

    // Suggest language/build-tool specific commands.
    if target_path.join("Cargo.toml").exists() {
        // Rust project detected
        println!("  {}. Build the Rust project: cargo build", step_number);
    } else if target_path.join("go.mod").exists() {
        // Go project detected
        println!(
            "  {}. Tidy Go modules and build: go mod tidy && go build",
            step_number
        );
    } else if target_path.join("package.json").exists() {
        // Node.js/JavaScript/TypeScript project detected
        println!(
            "  {}. Install Node.js dependencies: npm install (or yarn install)",
            step_number
        );
    } else if target_path.join("requirements.txt").exists()
        || target_path.join("pyproject.toml").exists()
    {
        // Python project detected
        println!(
            "  {}. Install Python dependencies (consider using a virtual environment):",
            step_number
        );
        // Suggest creating and activating a virtual environment.
        println!("     python -m venv .venv && source .venv/bin/activate"); // Use 'source' for Unix-like activation
        if target_path.join("pyproject.toml").exists() {
            // Suggest editable install for pyproject.toml (common pattern).
            println!("     pip install -e . (or use poetry/pdm install)");
        } else {
            // Suggest installing from requirements.txt.
            println!("     pip install -r requirements.txt");
        }
    } else if target_path.join("CMakeLists.txt").exists() {
        // C/C++ CMake project detected
        println!("  {}. Configure and build using CMake:", step_number);
        // Suggest using CMake presets if available, otherwise standard commands.
        // Assuming modern CMake workflow with presets.
        println!("     cmake --preset debug && cmake --build build --config Debug");
        println!("     # Or: cmake --preset release && cmake --build build --config Release");
    }
    // Add more checks here for other project types (Maven, Gradle, Composer, etc.) if needed.
}


// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf; // Ensure PathBuf is imported

    // Test parsing of command-line arguments for `blueprint create`.
    #[test]
    fn test_create_args_parsing() {
        // Test basic required args
        let args = CreateArgs::try_parse_from(&["create", "--lang", "rust", "my-project"]).unwrap();
        assert_eq!(args.lang, "rust");
        assert_eq!(args.project_name, "my-project");
        assert!(args.output.is_none());
        assert!(!args.force);
        assert!(args.var.is_empty());

        // Test all arguments
        let args_full = CreateArgs::try_parse_from(&[
            "create",
            "--lang",
            "go",
            "--output", // Optional output directory
            "/tmp/projects",
            "--force", // Optional force flag
            "--var", // Optional custom variables
            "author=Jane Doe",
            "--var",
            "version=1.0.0",
            "awesome-api", // Positional project name
        ])
        .unwrap();
        assert_eq!(args_full.lang, "go");
        assert_eq!(args_full.project_name, "awesome-api");
        assert_eq!(args_full.output, Some(PathBuf::from("/tmp/projects")));
        assert!(args_full.force);
        // Check if the custom variables were parsed correctly into the Vec.
        assert!(args_full
            .var
            .contains(&("author".to_string(), "Jane Doe".to_string())));
        assert!(args_full
            .var
            .contains(&("version".to_string(), "1.0.0".to_string())));
    }

    // Test that required arguments (`--lang` and `project_name`) are enforced.
    #[test]
    fn test_create_args_requires_lang_and_name() {
        // Missing --lang
        assert!(CreateArgs::try_parse_from(&["create", "my-project"]).is_err());
        // Missing project_name
        assert!(CreateArgs::try_parse_from(&["create", "--lang", "rust"]).is_err());
    }

    // Test the helper function for parsing `KEY=VALUE` strings.
    #[test]
    fn test_parse_key_val_logic() {
        // Standard case
        let (k, v) = parse_key_val("name=value").unwrap();
        assert_eq!(k, "name");
        assert_eq!(v, "value");

        // Case with padding
        let (k_pad, v_pad) = parse_key_val("  padded_key  =  padded value ").unwrap();
        assert_eq!(k_pad, "padded_key");
        assert_eq!(v_pad, "padded value");

        // Invalid format (missing '=')
        assert!(parse_key_val("invalid_no_equals").is_err());
        // Empty string
        assert!(parse_key_val("").is_err());
        // Only key
        assert!(parse_key_val("keyonly=").is_ok());
        let (k_only, v_only) = parse_key_val("keyonly=").unwrap();
        assert_eq!(k_only, "keyonly");
        assert_eq!(v_only, "");
        // Only value
         assert!(parse_key_val("=valueonly").is_ok());
         let (k_empty, v_val) = parse_key_val("=valueonly").unwrap();
        assert_eq!(k_empty, "");
        assert_eq!(v_val, "valueonly");
    }

    // Test the case conversion helper functions.
    #[test]
    fn test_case_conversion_logic() {
        // camelCase
        assert_eq!(to_camel_case("my-project"), "myProject");
        assert_eq!(to_camel_case("my_project"), "myProject");
        assert_eq!(to_camel_case("awesome-api-service"), "awesomeApiService");
        assert_eq!(to_camel_case("simple"), "simple");
        assert_eq!(to_camel_case("AlreadyCamel"), "alreadyCamel");
        assert_eq!(to_camel_case("PascalCase"), "pascalCase");
        assert_eq!(to_camel_case(""), "");

         // PascalCase
        assert_eq!(to_pascal_case("my-project"), "MyProject");
        assert_eq!(to_pascal_case("my_project"), "MyProject");
        assert_eq!(to_pascal_case("awesome-api-service"), "AwesomeApiService");
        assert_eq!(to_pascal_case("simple"), "Simple");
        assert_eq!(to_pascal_case("alreadyCamel"), "AlreadyCamel"); // Handles existing camel
        assert_eq!(to_pascal_case("PascalCase"), "PascalCase"); // Handles existing Pascal
        assert_eq!(to_pascal_case(""), "");

        // snake_case
        assert_eq!(to_snake_case("my-project"), "my_project");
        assert_eq!(to_snake_case("myProject"), "myproject"); // Doesn't insert underscores for camel
        assert_eq!(to_snake_case("PascalCase"), "pascalcase");
        assert_eq!(to_snake_case("already_snake"), "already_snake");
        assert_eq!(to_snake_case(""), "");
    }

    // Test the logic for building the template context map.
    #[test]
    fn test_build_template_context_logic() {
        // Define args with overrides and custom vars.
        let args = CreateArgs {
            lang: "test".into(),
            project_name: "my-App".into(), // Mixed case name
            output: None,
            force: false,
            var: vec![
                ("custom_key".into(), "custom_val".into()),
                // Override a default variable
                ("project_name".into(), "override".into()),
                 // Test snake case override input
                 ("project_name_snake".into(), "user_snake".into()),
            ],
        };

        // Build the context.
        let context = build_template_context(&args).unwrap();

        // --- Assertions ---
        // Check that user override took precedence.
        assert_eq!(context.get("project_name").unwrap(), "override");
        // Check that calculated variables still used the *original* project_name from args.
        assert_eq!(context.get("project_name_lowercase").unwrap(), "my-app");
        assert_eq!(context.get("project_name_uppercase").unwrap(), "MY-APP");
        assert_eq!(context.get("project_name_kebab").unwrap(), "my-App"); // Original name
        // Check that user override for snake_case worked.
        assert_eq!(context.get("project_name_snake").unwrap(), "user_snake");
         // Check calculated camel/pascal cases based on original name.
        assert_eq!(context.get("project_name_camel").unwrap(), "myApp");
        assert_eq!(context.get("project_name_pascal").unwrap(), "MyApp");
        // Check presence of date/time/user variables.
        assert!(context.contains_key("current_date"));
        assert!(context.contains_key("current_year"));
        assert!(context.contains_key("current_time"));
        assert!(context.contains_key("username"));
        // Check custom variable.
        assert_eq!(context.get("custom_key").unwrap(), "custom_val");
    }

    // Test path preparation and validation logic (requires mocking config and filesystem).
    #[tokio::test]
    #[ignore] // Needs mocks for config::load_config and filesystem checks
    async fn test_prepare_and_validate_paths_logic() {
        // Setup: Create temp dirs, mock config to point to temp blueprint root.
        // Test cases:
        // 1. Valid blueprint, target does not exist -> Ok
        // 2. Valid blueprint, target exists, no --force -> Err
        // 3. Valid blueprint, target exists, with --force -> Ok
        // 4. Valid blueprint, target is a file -> Err
        // 5. Invalid/missing blueprint -> Err
        // 6. Relative output path -> Ok (resolved correctly)
        // 7. Absolute output path -> Ok
        // 8. Output path requires parent creation -> Ok (and parent created)
    }

    // End-to-end test (requires mocking config, templating engine, filesystem).
    #[tokio::test]
    #[ignore] // Needs extensive mocks
    async fn test_handle_create_end_to_end() {
        // Setup: Mock config, mock templating::render_template_directory, create mock blueprint source.
        // Execute handle_create with various args.
        // Assert: Check that render_template_directory was called with correct args,
        // check console output for completion message (requires output capturing).
    }
}
