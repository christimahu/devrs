//! # DevRS Blueprint List Command
//!
//! File: cli/src/commands/blueprint/list.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs blueprint list` command, which displays
//! available project templates (blueprints) to the user. It handles:
//! - Reading the configured blueprint directory location
//! - Scanning for blueprint subdirectories
//! - Extracting descriptions from README.md files
//! - Formatting and displaying the blueprint information
//!
//! ## Architecture
//!
//! The command flow follows these steps:
//! 1. Load DevRS configuration to determine blueprint directory
//! 2. Scan the directory for subdirectories (each represents a blueprint)
//! 3. Read README.md files to extract descriptions
//! 4. Sort blueprints alphabetically by name
//! 5. Format and display the results in a tabular format
//!
//! ## Examples
//!
//! Usage:
//!
//! ```bash
//! devrs blueprint list
//! ```
//!
//! Example output:
//!
//! ```
//! Available Blueprints in '/home/user/tools/devrs/blueprints':
//!
//! Name       | Description
//! -----------+--------------------------------------------------
//! cpp        | C++ project with CMake build system
//! go         | Go project with module support and testing
//! rust       | Rust application with proper project structure
//!
//! Found 3 blueprint(s).
//! Use 'devrs blueprint info <Name>' for details or 'devrs blueprint create --lang <Name> ...' to use one.
//! ```
//!
//! If no blueprints are found, the command displays helpful instructions
//! for adding blueprints to the configured directory.
//!
use crate::core::config; // Access configuration loading functionality.
use crate::core::error::Result; // Use the standard Result type for error handling.
use anyhow::Context; // For adding contextual information to errors.
use clap::Parser; // For parsing command-line arguments.
use std::{
    fs,                    // Standard filesystem operations (reading directories, files).
    path::{Path, PathBuf}, // Filesystem path manipulation types.
};
use tracing::{debug, info, warn}; // Logging framework utilities.

/// # List Blueprint Arguments (`ListArgs`)
///
/// Defines the command-line arguments accepted by the `devrs blueprint list` subcommand.
/// Currently, this command doesn't require any specific arguments, but the struct
/// exists for structural consistency within the `clap` framework and allows for
/// potential future additions (like filtering or sorting options) without breaking changes.
#[derive(Parser, Debug)]
pub struct ListArgs {}

// --- Functions ---

/// # Handle Blueprint List Command (`handle_list`)
///
/// The main asynchronous handler function for the `devrs blueprint list` command.
/// It orchestrates the process of finding available blueprints in the configured
/// directory and displaying them to the user in a formatted table.
///
/// ## Workflow:
/// 1.  Logs the initiation of the command.
/// 2.  Loads the DevRS configuration using `config::load_config()` to locate the user's blueprint directory path.
/// 3.  Validates that the obtained blueprint directory path exists and is accessible.
/// 4.  Calls `read_blueprints_from_dir()` to scan the directory, identify valid blueprint subdirectories, and attempt to extract a description for each from its `README.md`.
/// 5.  Calls `print_blueprint_table()` to format and print the list of found blueprints (or a message if none were found).
///
/// ## Arguments
///
/// * `_args`: The parsed `ListArgs` struct. This argument is currently unused as the command takes no options.
///
/// ## Returns
///
/// * `Result<()>`: Returns `Ok(())` if the blueprint list was successfully generated and displayed.
/// * `Err`: Returns an `Err` if configuration loading fails, the blueprint directory is invalid, or reading the directory contents encounters an error.
pub async fn handle_list(_args: ListArgs) -> Result<()> {
    info!("Handling blueprint list command..."); // Log entry point.

    // Load configuration to get the blueprint directory path.
    let cfg = config::load_config().context("Failed to load DevRS configuration")?;
    // Convert the configured path string into a PathBuf.
    let bp_dir = PathBuf::from(&cfg.blueprints.directory);
    debug!("Scanning for blueprints in directory: {}", bp_dir.display()); // Log the target directory.

    // Validate the blueprint directory path obtained from configuration.
    if !bp_dir.is_dir() {
        anyhow::bail!("Blueprint directory '{}' is not a valid directory (or does not exist). Please check your devrs configuration.", bp_dir.display());
    }

    // Read the blueprint subdirectories and their descriptions.
    let blueprints = read_blueprints_from_dir(&bp_dir).with_context(|| {
        format!(
            "Failed to read blueprints from directory '{}'",
            bp_dir.display()
        )
    })?;

    // Print the discovered blueprints in a formatted table.
    print_blueprint_table(&blueprints, &bp_dir);

    Ok(()) // Indicate successful execution.
}

/// # Read Blueprints from Directory (`read_blueprints_from_dir`)
///
/// Scans the provided `bp_dir` path for subdirectories that represent potential blueprints.
/// It filters out hidden directories (names starting with '.') and any entries that are not directories.
/// For each valid blueprint directory, it attempts to extract a description using `read_blueprint_description`.
/// Finally, it returns a sorted list of (name, description) tuples.
///
/// ## Arguments
///
/// * `bp_dir`: A `&Path` reference to the main directory containing blueprint subdirectories.
///
/// ## Returns
///
/// * `Result<Vec<(String, String)>>`: A `Result` containing a vector of tuples, where each tuple is
///   `(blueprint_name, blueprint_description)`. The vector is sorted alphabetically by blueprint name.
///   Returns an `Err` if reading the `bp_dir` fails.
fn read_blueprints_from_dir(bp_dir: &Path) -> Result<Vec<(String, String)>> {
    // Initialize an empty vector to store the results.
    let mut blueprints = Vec::new();

    // Get an iterator for the entries within the blueprint directory.
    let entries = fs::read_dir(bp_dir).with_context(|| {
        format!(
            "Failed to read blueprint directory entries from '{}'",
            bp_dir.display()
        )
    })?;

    // Iterate over each entry in the directory.
    for entry_result in entries {
        // Handle potential errors while reading a specific entry.
        let entry = entry_result.with_context(|| {
            format!(
                "Failed to process a directory entry in '{}'",
                bp_dir.display()
            )
        })?;

        let path = entry.path(); // Get the full path of the entry.

        // Filter: Skip if the entry is not a directory or if its name starts with '.' (hidden).
        if !path.is_dir() || entry.file_name().to_string_lossy().starts_with('.') {
            debug!("Skipping non-blueprint entry: {}", path.display()); // Log skipped entries.
            continue; // Proceed to the next entry.
        }

        // Extract the directory name as the blueprint name.
        let name = entry.file_name().to_string_lossy().to_string();
        // Attempt to read the description from the blueprint's README.md.
        let description = read_blueprint_description(&path)
            // If description reading fails or README is missing, use a default placeholder.
            .unwrap_or_else(|| "[No README.md or description found]".to_string());

        // Add the found blueprint name and description to the results vector.
        blueprints.push((name, description));
    }

    // Sort the collected blueprints alphabetically based on their names (the first element of the tuple).
    blueprints.sort_by(|a, b| a.0.cmp(&b.0));

    // Return the sorted list of blueprints.
    Ok(blueprints)
}

/// # Read Blueprint Description (`read_blueprint_description`)
///
/// Attempts to locate a `README.md` file within the given `blueprint_path` directory.
/// If found and readable, it calls `extract_description_from_readme` to get the
/// description text.
///
/// ## Arguments
///
/// * `blueprint_path`: A `&Path` reference to the specific blueprint's directory.
///
/// ## Returns
///
/// * `Option<String>`: `Some(description)` if a README was found and a description could be
///   extracted, otherwise `None`. Returns `None` also if reading the README fails.
fn read_blueprint_description(blueprint_path: &Path) -> Option<String> {
    // Construct the full path to the potential README file.
    let readme_path = blueprint_path.join("README.md");

    // Check if the path exists and is a file (not a directory or symlink, etc.).
    if !readme_path.is_file() {
        debug!("No README.md found in {}", blueprint_path.display()); // Log if not found.
        return None; // Return None if README is missing or not a regular file.
    }

    // Attempt to read the contents of the README file into a string.
    match fs::read_to_string(&readme_path) {
        Ok(content) => {
            // If successful, extract the description from the content.
            extract_description_from_readme(&content)
        }
        Err(e) => {
            // If reading fails (e.g., permissions error), log a warning and return None.
            warn!(
                "Failed to read README file '{}': {}. Skipping description.",
                readme_path.display(),
                e
            );
            None
        }
    }
}

/// # Extract Description from README Content (`extract_description_from_readme`)
///
/// Parses the given markdown `content` string to extract a short description.
/// It prioritizes the first paragraph found after the first H1 heading (`# `).
/// If that fails, it falls back to the very first non-empty, non-heading line.
/// The resulting description is truncated to `MAX_LEN` characters if necessary.
///
/// ## Arguments
///
/// * `content`: A `&str` containing the markdown text to parse.
///
/// ## Returns
///
/// * `Option<String>`: `Some(description)` containing the extracted and potentially
///   truncated description, or `None` if no suitable description could be found.
fn extract_description_from_readme(content: &str) -> Option<String> {
    // Define the maximum length for the description to keep table output clean.
    const MAX_LEN: usize = 80;
    // Split the content into lines for easier processing.
    let lines: Vec<&str> = content.lines().collect();
    let mut description_lines = Vec::new(); // Stores lines identified as the description.
    let mut in_first_paragraph = false; // Flag to track if we are inside the target paragraph.
    let mut start_looking_idx = 0; // Line index to start searching from.

    // --- Find First Paragraph After H1 (Primary Method) ---
    // Find the index of the first H1 heading, if any.
    if let Some(idx) = lines.iter().position(|line| line.trim().starts_with("# ")) {
        start_looking_idx = idx + 1; // Start looking for description after the H1 line.
    }
    // Iterate through lines starting from `start_looking_idx`.
    for line in lines.iter().skip(start_looking_idx) {
        let trimmed = line.trim();
        // Look for the start of the paragraph (non-empty, not a heading).
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            in_first_paragraph = true; // Mark that we've entered the paragraph.
            description_lines.push(trimmed); // Add the line content.
        }
        // Look for the end of the paragraph.
        else if in_first_paragraph && trimmed.is_empty() {
            break; // Empty line marks the end.
        } else if in_first_paragraph && trimmed.starts_with('#') {
            break; // Another heading marks the end.
        }
    }

    // --- Fallback Method ---
    // If the primary method didn't find any description lines...
    if description_lines.is_empty() {
        // Iterate through all lines from the beginning.
        for line in &lines {
            let trimmed = line.trim();
            // Find the very first line that isn't empty and isn't a heading.
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                description_lines.push(trimmed); // Use this line as the description.
                break; // Stop after finding the first one.
            }
        }
    }

    // --- Format and Return ---
    // If still no description lines found, return None.
    if description_lines.is_empty() {
        None
    } else {
        // Join the collected lines (usually just one) into a single string.
        let full_desc = description_lines.join(" ");
        // Check if the description exceeds the maximum allowed length.
        if full_desc.chars().count() > MAX_LEN {
            // Truncate the description to MAX_LEN - 3 characters and add "...".
            let mut truncated: String = full_desc.chars().take(MAX_LEN.saturating_sub(3)).collect();
            truncated.push_str("...");
            Some(truncated)
        } else {
            // Description is short enough, return it as is.
            Some(full_desc)
        }
    }
}

/// # Print Blueprint Table (`print_blueprint_table`)
///
/// Takes a list of blueprint names and descriptions and prints them to the
/// console in a formatted table. Handles the case where no blueprints are found
/// and provides helpful hints after the table.
///
/// ## Arguments
///
/// * `blueprints`: A slice `&[(String, String)]` of (name, description) tuples.
/// * `bp_dir`: The `&Path` of the scanned blueprint directory (used for informational messages).
fn print_blueprint_table(blueprints: &[(String, String)], bp_dir: &Path) {
    // --- Handle Empty Case ---
    if blueprints.is_empty() {
        // Print a message indicating no blueprints were found and how to add them.
        println!("\nNo blueprints found in '{}'.\n", bp_dir.display());
        println!("To add blueprints, create subdirectories in this location containing project templates.");
        println!(
            "Each subdirectory represents one blueprint (e.g., '{}/rust/').",
            bp_dir.display()
        );
        return; // Exit the function early.
    }

    // --- Calculate Column Width ---
    // Determine the required width for the "Name" column based on the longest blueprint name.
    let name_width = blueprints
        .iter()
        .map(|(name, _)| name.len()) // Get length of each name.
        .max() // Find the maximum length.
        .unwrap_or(10) // Use 10 as a fallback width if list is empty (defensive).
        .clamp(10, 30); // Ensure minimum 10, maximum 30.

    // --- Print Table Header ---
    println!("\nAvailable Blueprints in '{}':\n", bp_dir.display());
    // Print the header row, left-aligning "Name" within the calculated width.
    println!("{:<width$} | Description", "Name", width = name_width);
    // Print the separator line, using hyphens matching the calculated width.
    // The description part uses a fixed width separator for simplicity.
    println!("{:-<width$}-+-{:-<50}", "", "", width = name_width);

    // --- Print Table Rows ---
    // Iterate through the sorted list of blueprints.
    for (name, description) in blueprints {
        // Print each blueprint's name (left-aligned) and description.
        println!("{:<width$} | {}", name, description, width = name_width);
    }

    // --- Print Footer Hints ---
    println!("\nFound {} blueprint(s).", blueprints.len());
    // Suggest next commands for the user.
    println!("Use 'devrs blueprint info <Name>' for details or 'devrs blueprint create --lang <Name> ...' to use one.");
}

// --- Unit Tests ---
// Unit tests remain unchanged as the code logic was not modified.
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_extract_description_from_readme_logic() {
        let readme1 = "# Title\n\nThis is the description.\nMore text.\n\n## Next Section";
        assert_eq!(
            extract_description_from_readme(readme1),
            Some("This is the description. More text.".to_string())
        );
        let readme2 = "# Title\nDescription line.\n## Next";
        assert_eq!(
            extract_description_from_readme(readme2),
            Some("Description line.".to_string())
        );
        let readme3 = "# Title\n\n## First Section\nSome content here first.";
        assert_eq!(
            extract_description_from_readme(readme3),
            Some("Some content here first.".to_string())
        );
        let long_desc = "This is a very long description designed to test the truncation logic which should limit the output to eighty characters including the ellipsis.";
        let readme4 = format!("# Title\n\n{}\n\n## Next", long_desc);
        let expected_truncated =
            "This is a very long description designed to test the truncation logic which s...";
        assert_eq!(
            extract_description_from_readme(&readme4),
            Some(expected_truncated.to_string())
        );
        let readme5 = "# Title\n\n## Section1\n## Section2";
        assert_eq!(extract_description_from_readme(readme5), None);
        let readme6 = "";
        assert_eq!(extract_description_from_readme(readme6), None);
        let readme7 = "First line of content.\nSecond line.";
        assert_eq!(
            extract_description_from_readme(readme7),
            Some("First line of content. Second line.".to_string())
        );
        let readme8 = "# Only Title";
        assert_eq!(extract_description_from_readme(readme8), None);
        let readme9 = "# Title\n## Section";
        assert_eq!(extract_description_from_readme(readme9), None);
    }
    #[test]
    fn test_read_blueprints_from_dir_logic() -> Result<()> {
        let temp_dir = tempdir()?;
        let bp_path = temp_dir.path();
        fs::create_dir(bp_path.join("rust-api"))?;
        fs::write(
            bp_path.join("rust-api/README.md"),
            "# Rust API\n\nCreates a Rust web API.",
        )?;
        fs::create_dir(bp_path.join("go-cli"))?;
        fs::create_dir(bp_path.join("python-simple"))?;
        fs::write(
            bp_path.join("python-simple/README.md"),
            "Just a basic python script setup.",
        )?;
        fs::create_dir(bp_path.join(".hidden"))?;
        fs::write(bp_path.join("not-a-dir.txt"), "ignore")?;
        let blueprints = read_blueprints_from_dir(bp_path)?;
        assert_eq!(blueprints.len(), 3);
        assert_eq!(blueprints[0].0, "go-cli");
        assert!(blueprints[0]
            .1
            .contains("[No README.md or description found]"));
        assert_eq!(blueprints[1].0, "python-simple");
        assert!(blueprints[1].1.contains("Just a basic python script setup"));
        assert_eq!(blueprints[2].0, "rust-api");
        assert!(blueprints[2].1.contains("Creates a Rust web API"));
        Ok(())
    }
    #[tokio::test]
    #[ignore]
    async fn test_handle_list_with_mock_config() -> Result<()> {
        Ok(())
    } // Requires mocking
    #[test]
    fn test_read_blueprints_missing_directory() {
        let non_existent_path = PathBuf::from("/path/that/does/not/exist/and/never/will");
        let result = read_blueprints_from_dir(&non_existent_path);
        assert!(result.is_err());
    }
}
