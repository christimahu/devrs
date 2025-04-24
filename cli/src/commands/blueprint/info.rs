//! # DevRS Blueprint Info Command
//!
//! File: cli/src/commands/blueprint/info.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the `devrs blueprint info` command, which displays
//! detailed information about a specific project blueprint. It handles:
//! - Reading the blueprint directory from configuration
//! - Locating the specified blueprint template
//! - Parsing README.md for title, description, and usage notes
//! - Detecting project type and build system
//! - Displaying file structure with a tree view
//! - Presenting usage information and examples
//!
//! ## Architecture
//!
//! The command flow follows these steps:
//! 1. Load configuration to determine blueprint directory
//! 2. Resolve and validate the blueprint path
//! 3. Read and parse the README.md for metadata
//! 4. Detect project type and build system using the project_detector utility
//! 5. Generate a tree view of the directory structure
//! 6. Format and display the information in a structured format
//!
//! ## Examples
//!
//! Usage:
//!
//! ```bash
//! devrs blueprint info rust
//! ```
//!
//! Example output:
//!
//! ```
//! ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
//! ┃ 🔎 Blueprint Details: rust                                            ┃
//! ┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫
//! ┃ 🏷️  Title:         Rust Project Template
//! ┃ 📝 Description:   A minimal idiomatic Rust blueprint for creating new applications
//! ┣--------------------------------------------------------------------┫
//! ┃ 🛠️  Project Info:
//! ┃    - Type:          Rust
//! ┃    - Build System:  Cargo
//! ┃    - Docker Support: Yes
//! ...
//! ```
//!
//! The command provides a rich, formatted display of blueprint details,
//! helping users understand what each template provides before using it.
//!
use crate::commands::blueprint::utils::{
    project_detector::{self, ProjectInfo}, // Utility to detect project type/build system
    tree_printer,                          // Utility to print directory structure
};
use crate::core::config; // Access configuration loading
use crate::core::error::Result; // Standard result type
use anyhow::Context; // For adding context to errors
use clap::Parser; // For command-line argument parsing
use std::{
    fs, // Standard filesystem operations (reading files)
    path::{Path, PathBuf}, // Filesystem path types
};
use tracing::{debug, info, warn}; // Logging framework

/// # Blueprint Info Arguments (`InfoArgs`)
///
/// Defines the command-line arguments accepted by the `devrs blueprint info` subcommand.
#[derive(Parser, Debug)]
pub struct InfoArgs {
    /// The name of the blueprint to display information about.
    /// This name must correspond to a subdirectory within the configured
    /// main blueprints directory (e.g., if the blueprint is in `~/devrs/blueprints/rust`,
    /// the name required here is `rust`).
    blueprint_name: String, // Required positional argument
}

/// # Handle Blueprint Info Command (`handle_info`)
///
/// The main asynchronous handler function for the `devrs blueprint info` command.
/// It orchestrates the process of gathering and displaying detailed information
/// about the specified blueprint template.
///
/// ## Workflow:
/// 1.  Loads the DevRS configuration to find the root directory where blueprints are stored.
/// 2.  Constructs the full path to the requested blueprint directory (`blueprint_path`).
/// 3.  Validates that `blueprint_path` exists and is actually a directory.
/// 4.  Calls helper functions to:
///     * Read and parse the blueprint's `README.md` file (if it exists) for title, description, and usage notes.
///     * Detect the project type (e.g., Rust, Go) and build system (e.g., Cargo, Go Modules) using the `project_detector` utility.
///     * Check for the presence of a `Dockerfile` (or template variant) to indicate Docker support.
/// 5.  Calls printing helper functions (`print_project_info`, `print_file_structure`, `print_usage_info`)
///     to format and display the gathered information to the console in a structured box layout.
///
/// ## Arguments
///
/// * `args` - The parsed `InfoArgs` struct containing the `blueprint_name`.
///
/// ## Returns
///
/// * `Result<()>` - Returns `Ok(())` if the information was successfully gathered and displayed.
/// * `Err` - Returns an error if configuration loading fails, the blueprint path is invalid,
///   or if there are issues reading the blueprint's contents (e.g., file system errors,
///   failure during tree printing).
pub async fn handle_info(args: InfoArgs) -> Result<()> {
    info!(
        "Handling blueprint info command for '{}'...",
        args.blueprint_name
    );

    // --- Path Resolution and Validation ---
    // Load global config to find the blueprints base directory.
    let cfg = config::load_config().context("Failed to load DevRS configuration")?;
    // Get the path (already expanded to absolute during config load).
    let bp_root_dir = PathBuf::from(&cfg.blueprints.directory);

    // Create the full path to the specific blueprint directory.
    let blueprint_path = bp_root_dir.join(&args.blueprint_name);
    debug!("Looking for blueprint at: {}", blueprint_path.display());

    // Ensure the derived blueprint path exists.
    if !blueprint_path.exists() {
        anyhow::bail!(
            "Blueprint '{}' not found in '{}'. Run 'devrs blueprint list' to see available blueprints.",
            args.blueprint_name,
            bp_root_dir.display()
        );
    }
    // Ensure the path points to a directory, not a file.
    if !blueprint_path.is_dir() {
        anyhow::bail!(
            "Path '{}' exists but is not a directory.",
            blueprint_path.display()
        );
    }

    // --- Gather Blueprint Information ---

    // Attempt to read and parse the README.md file within the blueprint directory.
    let readme_path = blueprint_path.join("README.md");
    let (title, description, usage_notes) =
        read_and_parse_readme(&readme_path).with_context(|| {
            format!(
                "Failed to process README for blueprint '{}'",
                args.blueprint_name
            )
        })?;

    // Detect the project type (e.g., Rust) and build system (e.g., Cargo)
    // by analyzing files within the blueprint directory.
    let project_info = project_detector::detect_project_type(&blueprint_path); //

    // Check for the presence of a Dockerfile (or common template variations)
    // to indicate whether the blueprint includes Docker support.
    let has_dockerfile = [
        "Dockerfile",
        "Dockerfile.template",
        "Dockerfile.tmpl",
        "Dockerfile.tera",
    ]
    .iter()
    .any(|name| blueprint_path.join(name).exists());

    // --- Print Formatted Output ---
    // Uses Unicode box drawing characters for a visually organized presentation.
    println!("\n┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓");
    // Print header with blueprint name, padded with spaces for alignment.
    println!(
        "┃ 🔎 Blueprint Details: {} {}",
        args.blueprint_name,
        " ".repeat(51usize.saturating_sub(args.blueprint_name.len())) // Pad dynamically
    );
    println!("┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫");

    // Print Title and Description section.
    println!("┃ 🏷️  Title:         {}", title);
    println!("┃ 📝 Description:   {}", description);
    println!("┣--------------------------------------------------------------------┫");

    // Print Project Info section.
    print_project_info(&project_info, has_dockerfile);
    println!("┣--------------------------------------------------------------------┫");

    // Print File Structure section (uses the tree_printer utility).
    print_file_structure(&blueprint_path, &args.blueprint_name)?;
    println!("┣--------------------------------------------------------------------┫");

    // Print Usage section (standard commands + notes from README).
    print_usage_info(&args.blueprint_name, &usage_notes);

    println!("┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛");

    Ok(())
}

/// # Read and Parse README (`read_and_parse_readme`)
///
/// Attempts to read the `README.md` file at the given path and extracts key
/// information: the main title (H1), the first paragraph description, and any
/// dedicated usage/setup section.
///
/// If the `README.md` file is not found or cannot be read, it returns default placeholder
/// values and logs a warning.
///
/// ## Arguments
///
/// * `readme_path` - The `&Path` to the `README.md` file to process.
///
/// ## Returns
///
/// * `Result<(String, String, Option<String>)>` - A tuple containing:
///     1. `String`: The extracted title (or a default).
///     2. `String`: The extracted description (or a default).
///     3. `Option<String>`: The extracted usage notes, if a relevant section was found.
///   Returns `Err` if reading the file fails (other than not found).
fn read_and_parse_readme(readme_path: &Path) -> Result<(String, String, Option<String>)> {
    // Check if the README file exists and is a file.
    if !readme_path.is_file() {
        // Log a warning if README is missing.
        warn!(
            "No README.md found at {}. Using default info.",
            readme_path.display()
        );
        // Return default values when README is absent.
        return Ok((
            "Blueprint Information".to_string(), // Default title
            "No README.md found.".to_string(),   // Default description
            None,                                // No usage notes
        ));
    }

    // Read the content of the README file into a string.
    let content = fs::read_to_string(readme_path)
        .with_context(|| format!("Failed to read README file: {}", readme_path.display()))?;

    // Extract title and description using a helper function.
    let (title, description) = extract_title_and_description(&content);
    // Extract usage section using another helper function.
    let usage = extract_usage_from_readme(&content);

    // Return the extracted information.
    Ok((title, description, usage))
}

/// # Extract Title and Description (`extract_title_and_description`)
///
/// Parses the content of a README markdown string to find the main title
/// (expected to be the first H1 heading, like `# Title`) and the first
/// paragraph of text that appears after the title (or the very first paragraph
/// if no H1 is found).
///
/// ## Arguments
///
/// * `content` - A string slice (`&str`) containing the markdown content of the README.
///
/// ## Returns
///
/// * `(String, String)` - A tuple containing:
///     1. `String`: The extracted title (trimmed, defaults to "Blueprint Information").
///     2. `String`: The extracted first paragraph description (trimmed, defaults to a placeholder).
fn extract_title_and_description(content: &str) -> (String, String) {
    let lines: Vec<&str> = content.lines().collect();
    let mut title = "Blueprint Information".to_string(); // Default if no H1 found
    let mut description_lines = Vec::new();
    let mut in_first_paragraph = false;
    let mut start_looking_idx = 0; // Index in `lines` to start looking for the description

    // --- Find Title (First H1) ---
    // Find the first line that starts with "# " (markdown H1 syntax).
    if let Some((idx, line)) = lines
        .iter()
        .enumerate()
        .find(|(_, l)| l.trim().starts_with("# "))
    {
        // If found, extract the text after "# ", trim whitespace, and assign as title.
        title = line.trim().trim_start_matches("# ").trim().to_string();
        // Start looking for the description *after* the title line.
        start_looking_idx = idx + 1;
    }

    // --- Find Description (First Paragraph After Title/Start) ---
    // Iterate through lines starting from after the title (or from the beginning if no title found).
    for line in lines.iter().skip(start_looking_idx) {
        let trimmed = line.trim();
        // Check if the line is part of the first paragraph:
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            // Found a non-empty, non-heading line - start of the paragraph.
            in_first_paragraph = true;
            description_lines.push(trimmed); // Add the line content.
        }
        // Check for the end of the first paragraph:
        else if in_first_paragraph && trimmed.is_empty() {
            break; // An empty line signifies the end of the paragraph.
        } else if in_first_paragraph && trimmed.starts_with('#') {
            break; // Hitting another heading also signifies the end.
        }
        // Ignore empty lines or headings before the first paragraph starts.
    }

    // --- Finalize Description ---
    let description = if description_lines.is_empty() {
        // If no paragraph was captured (e.g., README only has headings or is empty),
        // try a simpler fallback: find the very first non-empty, non-heading line.
        lines
            .iter()
            .map(|l| l.trim()) // Trim each line
            .find(|l| !l.is_empty() && !l.starts_with('#')) // Find the first suitable line
            .map(|l| l.to_string()) // Convert to String if found
            .unwrap_or_else(|| "No description available in README.".to_string()) // Default if still nothing
    } else {
        // Join the captured lines into a single string, separated by spaces.
        description_lines.join(" ")
    };

    (title, description)
}

/// # Extract Usage Section (`extract_usage_from_readme`)
///
/// Parses the content of a README markdown string to find a section related to
/// usage, setup, installation, or getting started. It looks for specific headings
/// (case-insensitive) and extracts all content following that heading until the
/// next heading of the same or higher level is encountered.
///
/// ## Target Headings (Keywords searched for, case-insensitive):
/// - "usage"
/// - "setup"
/// - "getting started"
/// - "installation"
/// - "building"
/// - "running"
///
/// ## Arguments
///
/// * `content` - A string slice (`&str`) containing the markdown content of the README.
///
/// ## Returns
///
/// * `Option<String>` - `Some(String)` containing the extracted usage notes (preserving
///   original indentation and internal newlines) if a target section is found,
///   otherwise `None`.
fn extract_usage_from_readme(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut usage_lines = Vec::new(); // Stores lines belonging to the usage section
    let mut in_usage_section = false; // Flag: are we currently inside a target section?
    let mut current_section_level = 0; // Markdown heading level (#, ##, ###) of the current target section

    for line in lines {
        let trimmed = line.trim();

        // Check if the line is a markdown heading
        if trimmed.starts_with('#') {
            // Determine heading level (number of '#')
            let level = trimmed.chars().take_while(|&c| c == '#').count();
            // Extract heading text, convert to lowercase for case-insensitive matching
            let heading_text = trimmed.trim_start_matches('#').trim().to_lowercase();

            // Check if the heading text contains any of our target keywords
            let is_target_section = heading_text.contains("usage")
                || heading_text.contains("setup")
                || heading_text.contains("getting started")
                || heading_text.contains("installation")
                || heading_text.contains("building")
                || heading_text.contains("running");

            if in_usage_section {
                // If we are already inside a target section:
                // Check if this new heading marks the end of our section.
                // The section ends if we hit another heading of the *same or higher* level (e.g., H2 ends at next H2 or H1).
                if level <= current_section_level {
                    break; // End of the target section found.
                }
                // If it's a lower-level heading (e.g., H3 inside an H2 section),
                // include it as part of the usage notes.
                if !trimmed.is_empty() {
                    usage_lines.push(line); // Keep original line including heading marks
                }
            } else if is_target_section {
                // Found the start of a new target section.
                in_usage_section = true;
                current_section_level = level;
                // Do *not* add the heading line itself to `usage_lines`.
            }
        }
        // If the line is not a heading:
        else if in_usage_section && !trimmed.is_empty() {
            // If we are inside a target section and the line has content, add it.
            usage_lines.push(line); // Preserve original indentation by adding the whole line.
        } else if in_usage_section && trimmed.is_empty() && !usage_lines.is_empty() {
            // If inside a target section, keep empty lines *after* the first line
            // to preserve formatting like in code blocks.
            usage_lines.push(line);
        }
    }

    // Clean up any trailing empty lines that might have been added.
    while let Some(last) = usage_lines.last() {
        if last.trim().is_empty() {
            usage_lines.pop();
        } else {
            break;
        }
    }

    // If we collected any lines, join them back with newlines and return Some.
    if usage_lines.is_empty() {
        None
    } else {
        Some(usage_lines.join("\n"))
    }
}

/// # Print Project Info Section (`print_project_info`)
///
/// Formats and prints the "Project Info" section of the `blueprint info` output,
/// including the detected project type, build system, and Docker support status.
/// Adheres to the box layout style.
///
/// ## Arguments
///
/// * `project_info` - A reference to the `ProjectInfo` struct containing detected type/system.
/// * `has_dockerfile` - A boolean indicating if a Dockerfile was found.
fn print_project_info(project_info: &ProjectInfo, has_dockerfile: bool) {
    // Print the section header.
    println!("┃ 🛠️  Project Info:");
    // Print detected project type.
    println!("┃    - Type:          {}", project_info.project_type);
    // Print detected build system.
    println!("┃    - Build System:  {}", project_info.build_system);
    // Indicate Docker support based on the flag.
    println!(
        "┃    - Docker Support: {}",
        if has_dockerfile { "Yes" } else { "No" }
    );
}

/// # Print File Structure Section (`print_file_structure`)
///
/// Generates and prints the "File Structure" section of the `blueprint info` output.
/// It uses the `tree_printer` utility to create a visual tree of the blueprint's
/// directory contents and formats it within the info box layout.
///
/// ## Arguments
///
/// * `blueprint_path` - The `&Path` to the blueprint directory.
/// * `blueprint_name` - The `&str` name of the blueprint (used as the root label in the tree).
///
/// ## Returns
///
/// * `Result<()>` - `Ok(())` if successful, or propagates an `Err` from the `tree_printer`.
fn print_file_structure(blueprint_path: &Path, blueprint_name: &str) -> Result<()> {
    println!("┃ 📁 File Structure:");
    // Attempt to generate the directory tree string.
    match tree_printer::print_directory_tree_to_string(blueprint_path, blueprint_name) { //
        Ok(tree_string) => {
            // If successful, iterate through the lines of the tree string.
            for line in tree_string.lines() {
                // Print each line, indented to fit within the info box border.
                println!("┃    {}", line);
            }
        }
        Err(e) => {
            // If tree generation fails, log a warning and print an error message.
            warn!("Could not display file structure: {}", e);
            println!("┃    [Error displaying file structure]");
            // Propagate the error up to the main handler.
            return Err(e).context("Failed to display blueprint file structure");
        }
    }
    Ok(())
}

/// # Print Usage Info Section (`print_usage_info`)
///
/// Formats and prints the "Usage" section of the `blueprint info` output.
/// It includes standard instructions on how to use `devrs blueprint create`
/// with the specific blueprint, followed by any additional usage notes
/// extracted from the blueprint's `README.md` file.
///
/// ## Arguments
///
/// * `blueprint_name` - The `&str` name of the blueprint.
/// * `usage_notes` - An `Option<String>` containing usage notes extracted from the README, if any.
fn print_usage_info(blueprint_name: &str, usage_notes: &Option<String>) {
    // Print standard usage instructions.
    println!("┃ 🚀 Usage:");
    println!("┃    To create a new project from this blueprint:");
    // Show the command format using the specific blueprint name.
    println!(
        "┃      devrs blueprint create --lang {} <your-project-name>",
        blueprint_name
    );
    println!("┃"); // Blank line for spacing
    println!("┃    Example:");
    // Provide a concrete example.
    println!(
        "┃      devrs blueprint create --lang {} my-awesome-app",
        blueprint_name
    );

    // If additional usage notes were extracted from the README, print them.
    if let Some(notes) = usage_notes {
        println!("┃"); // Blank line for spacing
        println!("┃ 📋 Additional Notes (from README.md):");
        // Iterate through the lines of the notes.
        for line in notes.trim().split('\n') {
            // Print each line indented and prefixed with '> '.
            println!("┃    > {}", line.trim_end()); // Trim trailing whitespace from notes lines
        }
    }
}


// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir; // For creating temporary directories in tests

    // --- Test Arg Parsing ---
    #[test]
    fn test_info_args_parsing() {
        // Test parsing a valid command with the required blueprint name.
        let args = InfoArgs::try_parse_from(&["info", "my-blueprint"]).unwrap();
        assert_eq!(args.blueprint_name, "my-blueprint");
    }

    #[test]
    fn test_info_args_requires_name() {
        // Test that parsing fails if the required blueprint name is missing.
        let result = InfoArgs::try_parse_from(&["info"]);
        assert!(result.is_err(), "Should fail without blueprint name");
    }

    // --- Test README Parsing Helpers ---
    #[test]
    fn test_extract_title_and_description_parsing() {
        // Test Case 1: Standard title and description
        let readme1 = "# Title\n\nDesc line 1.\nDesc line 2.\n\n## Next";
        let (t1, d1) = extract_title_and_description(readme1);
        assert_eq!(t1, "Title");
        assert_eq!(d1, "Desc line 1. Desc line 2."); // Paragraph joined

        // Test Case 2: Title only, no description paragraph
        let readme2 = "# Only Title\n\n## Section";
        let (t2, d2) = extract_title_and_description(readme2);
        assert_eq!(t2, "Only Title");
        assert!(d2.contains("No description available")); // Fallback description

        // Test Case 3: No H1 title, description should be first non-empty/non-heading line
        let readme3 = "No title line.\nFirst paragraph.";
        let (t3, d3) = extract_title_and_description(readme3);
        assert!(t3.contains("Blueprint Information")); // Default title
        assert_eq!(d3, "No title line. First paragraph."); // Expect joined lines from fallback

        // Test Case 4: Title appears later, description follows it
        let readme4 = "First line\n# Title Later\n\nDesc after title";
        let (t4, d4) = extract_title_and_description(readme4);
        assert_eq!(t4, "Title Later");
        assert_eq!(d4, "Desc after title");

        // Test Case 5: Indented description paragraph
        let readme5 = "# Title\n\n    Indented paragraph.\n";
        let (t5, d5) = extract_title_and_description(readme5);
        assert_eq!(t5, "Title");
        assert_eq!(d5, "Indented paragraph."); // Trimmed whitespace
    }

    #[test]
    fn test_extract_usage_from_readme_parsing() {
        // Test Case 1: Basic Usage section
        let readme1 = "# Title\n\nDesc.\n\n## Usage\nRun this.\nThen that.\n\n## Other";
        assert_eq!(
            extract_usage_from_readme(readme1),
            Some("Run this.\nThen that.".to_string())
        );

        // Test Case 2: Installation section with subsection, ending at next H2
        let readme2 = "# Title\n\n## Installation\nInstall A.\nInstall B.\n### Sub Section\nMore details\n## Next Section\nfoo";
        assert_eq!(
            extract_usage_from_readme(readme2),
            // Includes the subsection heading and content
            Some("Install A.\nInstall B.\n### Sub Section\nMore details".to_string())
        );

        // Test Case 3: No target usage/setup section found
        let readme3 = "# Title\n\n## Features\nFeature list.";
        assert_eq!(extract_usage_from_readme(readme3), None);

        // Test Case 4: Usage section ends at next H2, includes empty line
        let readme4 = "# Title\n\n## Usage\nLine1\n\nLine2 after empty\n\n## Build\nBuild step.";
        assert_eq!(
            extract_usage_from_readme(readme4),
            Some("Line1\n\nLine2 after empty".to_string()) // Preserves internal empty line
        );

        // Test Case 5: Getting Started section with code block
        let readme5 = "# Title\n## Getting Started\nStep 1\n```bash\ncode example\n```\nStep 2";
        assert_eq!(
            extract_usage_from_readme(readme5),
            Some("Step 1\n```bash\ncode example\n```\nStep 2".to_string()) // Preserves code block
        );

        // Test Case 6: Heading contains "Usage" but isn't exactly "Usage"
        let readme6 = "# Title\n## Usage Instructions\nUse it wisely.";
        assert_eq!(
            extract_usage_from_readme(readme6),
            Some("Use it wisely.".to_string())
        );

        // Test Case 7: Empty Usage section
         let readme7 = "# Title\n## Usage\n\n## Next Section";
        assert_eq!(extract_usage_from_readme(readme7), None); // Empty section results in None

         // Test Case 8: Usage section at end of file
         let readme8 = "# Title\n## Running\nRun command foo\n";
         assert_eq!(extract_usage_from_readme(readme8), Some("Run command foo".to_string()));
    }

    // --- Test Handler Logic (Requires Mocks) ---
    // These tests demonstrate the structure but would need actual mocking
    // of `config::load_config`, `project_detector::detect_project_type`,
    // and `tree_printer::print_directory_tree_to_string` to run effectively.

    /// Placeholder test for the main handler logic with a valid blueprint.
    #[tokio::test]
    #[ignore] // Mocking is complex for unit tests here
    async fn test_handle_info_logic() -> Result<()> {
        let temp_dir = tempdir()?; // Create a temporary directory for the blueprint root
        let bp_root = temp_dir.path();

        // Create a mock blueprint structure inside the temp directory
        let bp_name = "test-bp";
        let bp_path = bp_root.join(bp_name);
        fs::create_dir_all(bp_path.join("src"))?;
        fs::write(
            bp_path.join("README.md"), // Create a README
            "# Test BP\n\nDescription here.\n\n## Usage\nRun it.",
        )?;
        fs::write(bp_path.join("src/main.rs"), "fn main() {}")?; // Add a source file
        fs::write(bp_path.join("Cargo.toml"), "[package]")?; // Add Rust marker file
        fs::write(bp_path.join("Dockerfile.template"), "FROM rust")?; // Add Docker marker file

        // --- Mocking Setup (Conceptual) ---
        // Need to mock:
        // 1. config::load_config() -> returns Ok(Config pointing to bp_root)
        // 2. project_detector::detect_project_type(&bp_path) -> returns Ok(ProjectInfo { type: "Rust", system: "Cargo" })
        // 3. tree_printer::print_directory_tree_to_string(&bp_path, bp_name) -> returns Ok("mock tree output")

        // --- Execution ---
        let args = InfoArgs {
            blueprint_name: bp_name.to_string(),
        };
        let result = handle_info(args).await;

        // --- Assertion ---
        // For now, just assert it runs without error (replace with stdout capture/mock verification later)
        assert!(result.is_ok());
        // TODO: Capture stdout and verify formatted output contains expected title, description,
        // project info (Rust/Cargo/Docker Yes), usage notes, and the mock tree output.

        Ok(())
    }

    /// Placeholder test for when the specified blueprint is not found.
    #[tokio::test]
    #[ignore] // Mocking required
    async fn test_handle_info_missing_blueprint() {
        // --- Mocking Setup (Conceptual) ---
        // Mock config::load_config() -> returns Ok(Config pointing to bp_root)
        // Note: project_detector and tree_printer won't be called if path validation fails.

        // --- Execution ---
        let args = InfoArgs {
            // Use a name that definitely won't exist in the temp dir
            blueprint_name: "nonexistent-bp".to_string(),
        };
        let result = handle_info(args).await;

        // --- Assertion ---
        assert!(result.is_err()); // Expect an error because the blueprint path doesn't exist
        // Check that the error message indicates the blueprint wasn't found.
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
