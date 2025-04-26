//! # DevRS Directory Tree Printer
//!
//! File: cli/src/commands/blueprint/utils/tree_printer.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module provides functionality to print a formatted directory tree structure,
//! similar to the output of the `tree` command-line utility. It's used by
//! `devrs blueprint info` to visualize the contents of a blueprint template.
//!
//! ## Architecture
//!
//! The tree printer implements several features:
//!
//! - Recursive directory traversal
//! - Indentation and connector lines (├──, └──, │)
//! - Skipping of hidden files/directories (those starting with '.')
//! - Basic cycle detection for symlinks (using canonical paths)
//! - Bold formatting for directory names (using ANSI escape codes)
//!
//! ## Usage
//!
//! The module offers two primary functions:
//!
//! ```rust
//! // Print directly to stdout
//! print_directory_tree(&path, "project_name")?;
//!
//! // Return as a string for further processing
//! let tree_string = print_directory_tree_to_string(&path, "project_name")?;
//! ```
//!
//! Example output:
//!
//! ```
//! project_name/
//! ├── src/
//! │   ├── main.rs
//! │   └── lib.rs
//! ├── tests/
//! │   └── integration_test.rs
//! ├── Cargo.toml
//! └── README.md
//! ```
//!
use crate::core::error::Result; // Use anyhow Result for error propagation
use anyhow::{anyhow, Context}; // For adding context to errors
use std::{
    collections::HashSet,   // Used for cycle detection
    fmt::Write as FmtWrite, // Import the Write trait for string building, aliased to avoid conflicts
    fs,                     // Standard filesystem operations
    path::{Path, PathBuf},  // Filesystem path types
};
use tracing::{debug, warn}; // For logging debug information and warnings

// --- Constants for Tree Drawing ---
// These constants define the Unicode characters used to draw the tree structure,
// ensuring a consistent visual appearance similar to the standard `tree` command.

/// Connector for intermediate items in a directory listing ("T" shape).
const TEE: &str = "├── ";
/// Connector for the last item in a directory listing ("L" shape).
const ELBOW: &str = "└── ";
/// Vertical line used for ongoing indentation levels.
const PIPE: &str = "│   ";
/// Spacer used for indentation levels after the last item has been printed.
const SPACER: &str = "    ";
/// ANSI escape code to start bold text formatting (for directories).
const BOLD_START: &str = "\x1b[1m";
/// ANSI escape code to reset text formatting (ends bolding).
const BOLD_END: &str = "\x1b[0m";

/// # Directory Entry Struct (`DirEntry`)
///
/// A temporary internal struct used to hold information about a single file or
/// directory entry read from the filesystem. This allows collecting entries
/// and sorting them (directories first, then alphabetically) before processing
/// them for the tree output.
struct DirEntry {
    /// The full path to the filesystem entry.
    path: PathBuf,
    /// The display name (file or directory name) of the entry.
    name: String,
    /// Flag indicating whether the entry is a directory.
    is_dir: bool,
}

/// # Print Directory Tree to Stdout (`print_directory_tree`)
///
/// A convenience function that generates the directory tree string using
/// `print_directory_tree_to_string` and then prints the resulting string
/// directly to standard output.
///
/// ## Arguments
///
/// * `root_path` - The `&Path` to the directory whose structure should be printed.
/// * `display_name` - The string slice (`&str`) representing the name to use for the root directory in the output.
///
/// ## Returns
///
/// * `Result<()>` - Returns `Ok(())` if the tree was generated and printed successfully,
///   otherwise returns an `Err` containing the error details (e.g., path not found, read error).
#[allow(dead_code)] // Keep this public function available, even if only the string version is currently used internally.
pub fn print_directory_tree(root_path: &Path, display_name: &str) -> Result<()> {
    // Generate the tree as a string first.
    let tree_string = print_directory_tree_to_string(root_path, display_name)?;
    // Print the generated string to stdout.
    println!("{}", tree_string);
    Ok(())
}

/// # Generate Directory Tree String (`print_directory_tree_to_string`)
///
/// The main public function to generate a visual tree representation of a
/// directory's structure as a `String`. It validates the input path, initializes
/// the cycle detection mechanism, prints the root node, and starts the recursive
/// traversal using `walk_and_build_string`.
///
/// ## Arguments
///
/// * `root_path` - The `&Path` to the directory whose structure should be generated.
/// * `display_name` - The string slice (`&str`) representing the name to use for the root directory.
///
/// ## Returns
///
/// * `Result<String>` - Returns `Ok(String)` containing the formatted tree if successful,
///   otherwise returns an `Err` containing the error details.
pub fn print_directory_tree_to_string(root_path: &Path, display_name: &str) -> Result<String> {
    // --- Input Validation ---
    if !root_path.exists() {
        // Return an error if the specified root path doesn't exist.
        anyhow::bail!(
            "Cannot print tree: Path '{}' does not exist.",
            root_path.display()
        );
    }
    if !root_path.is_dir() {
        // Return an error if the specified path is not a directory.
        anyhow::bail!(
            "Cannot print tree: Path '{}' is not a directory.",
            root_path.display()
        );
    }

    // --- Initialization ---
    // `visited`: A HashSet to store the canonical paths of visited directories.
    // This is used to detect cycles caused by symbolic links pointing back
    // to an ancestor directory, preventing infinite recursion.
    let mut visited = HashSet::new();
    // `output`: The string buffer where the tree representation will be built.
    let mut output = String::new();

    // Attempt to get the canonical (absolute, symlink-resolved) path of the root.
    // Add it to the visited set to prevent the root from being part of a cycle check later.
    match root_path.canonicalize() {
        Ok(canonical_root) => {
            // Store the resolved path.
            visited.insert(canonical_root);
        }
        Err(e) => {
            // If canonicalization fails (e.g., permissions, broken link component),
            // log a warning. Cycle detection might be less effective.
            warn!(
                "Could not canonicalize root path '{}': {}. Cycle detection might be affected.",
                root_path.display(),
                e
            );
            // Continue execution without adding to visited set in this case.
        }
    };

    // --- Start Tree Generation ---
    // Print the root directory name, applying bold formatting using ANSI codes.
    // The `writeln!` macro requires the `std::fmt::Write` trait (aliased as `FmtWrite`).
    writeln!(output, "{}{}/{}", BOLD_START, display_name, BOLD_END)
        .map_err(|e| anyhow!(e).context("Failed to write root directory name"))?;

    // Initiate the recursive walk starting from the root directory.
    // `current_prefix` starts empty.
    // `visited` contains the canonical root path (if resolved).
    // `output` is the buffer to write to.
    walk_and_build_string(
        root_path,          // Start directory
        &mut String::new(), // Initial prefix (empty)
        &mut visited,       // Set for cycle detection
        &mut output,        // Output buffer
    )
    .context("Failed while generating directory tree structure string")?; // Add context on error

    // Return the completed tree string.
    Ok(output)
}

/// # Recursive Tree Walker (`walk_and_build_string`)
///
/// This is the core recursive function that traverses the directory structure.
/// For each directory, it reads entries, sorts them, and then iterates through,
/// printing the correct prefix, connector, and name for each entry. If an entry
/// is a directory, it recursively calls itself to print the subtree, managing
/// indentation and cycle detection.
///
/// ## Arguments
///
/// * `dir` - The `&Path` of the current directory being processed.
/// * `current_prefix` - A mutable `&mut String` holding the indentation and connector lines
///   (e.g., "│   ├── ") needed for the current level. This is built up during recursion.
/// * `visited` - A mutable `&mut HashSet<PathBuf>` passed down through recursion to track
///   visited directories (by canonical path) for cycle detection.
/// * `output` - A mutable reference `&mut dyn FmtWrite` (typically a `String`) where the
///   tree output is written. Using the trait allows flexibility (e.g., writing directly to stdout).
///
/// ## Returns
///
/// * `Result<()>` - Returns `Ok(())` on successful traversal of this level, or an `Err` if
///   reading directory entries fails or writing to the output buffer fails.
fn walk_and_build_string(
    dir: &Path,
    current_prefix: &mut String,
    visited: &mut HashSet<PathBuf>,
    output: &mut dyn FmtWrite,
) -> Result<()> {
    // Read and sort the entries (directories first, then files, alphabetically)
    // This ensures a consistent and readable tree structure.
    let entries = read_and_sort_dir_entries(dir)?;

    let num_entries = entries.len();
    // Iterate through the sorted entries with their index.
    for (index, entry) in entries.into_iter().enumerate() {
        // Determine if this is the last entry in the current directory listing.
        let is_last_entry = index == num_entries - 1;

        // --- Print Entry Line ---
        // 1. Print the accumulated prefix for the current indentation level.
        write!(output, "{}", current_prefix)?;

        // 2. Print the appropriate connector based on whether it's the last entry.
        let connector = if is_last_entry { ELBOW } else { TEE }; // "└── " or "├── "
        write!(output, "{}", connector)?;

        // 3. Print the entry name. Apply bold formatting if it's a directory.
        if entry.is_dir {
            writeln!(output, "{}{}{}", BOLD_START, entry.name, BOLD_END)?;
        } else {
            writeln!(output, "{}", entry.name)?;
        }

        // --- Handle Recursion for Directories ---
        if entry.is_dir {
            // Determine the prefix component to add for the *next* level of recursion.
            // If this was the last entry at the current level, use a spacer; otherwise, use a pipe.
            let prefix_component = if is_last_entry { SPACER } else { PIPE }; // "    " or "│   "
                                                                              // Append this component to the prefix *before* the recursive call.
            current_prefix.push_str(prefix_component);

            // --- Cycle Detection ---
            let mut skip_recursion = false;
            // Try to get the canonical path of the directory entry.
            match entry.path.canonicalize() {
                Ok(canonical_path) => {
                    // Check if this canonical path has already been visited in the current traversal path.
                    if visited.contains(&canonical_path) {
                        // Cycle detected! Log a warning and prepare to skip recursion.
                        warn!(
                            "Detected symlink cycle or duplicate traversal for: '{}'. Skipping subtree.",
                            entry.path.display()
                        );
                        // Print a marker in the output to indicate the cycle.
                        // Calculate the correct prefix for the cycle message line.
                        let cycle_prefix = format!(
                            "{}{}",
                            current_prefix
                                .trim_end_matches(PIPE)
                                .trim_end_matches(SPACER),
                            SPACER // Ensure consistent indentation
                        );
                        // Write the cycle indicator line.
                        writeln!(output, "{}{} -> [CYCLE DETECTED]", cycle_prefix, ELBOW)?;
                        skip_recursion = true; // Mark this directory to be skipped.
                    } else {
                        // No cycle detected. Add the canonical path to the visited set
                        // *before* recursing into it.
                        visited.insert(canonical_path);
                        // Note: We technically don't need to remove from `visited` upon returning,
                        // as we only care about cycles *within* a single branch of the tree walk.
                        // If a directory is legitimately linked multiple times from different
                        // places, the first traversal will print it, and subsequent ones will
                        // hit the cycle detection.
                    }
                }
                Err(e) => {
                    // Canonicalization failed (e.g., permissions, broken symlink).
                    // Log a warning, as cycle detection won't work reliably for this path.
                    warn!(
                        "Could not canonicalize path '{}': {}. Skipping cycle check.",
                        entry.path.display(),
                        e
                    );
                    // Optional: Could choose to skip recursion here (`skip_recursion = true;`)
                    // for safety, but currently allows traversal if readable.
                }
            }

            // --- Recursive Call ---
            // If no cycle was detected (or canonicalization failed but we're proceeding),
            // recursively call this function for the subdirectory.
            if !skip_recursion {
                walk_and_build_string(&entry.path, current_prefix, visited, output)?;
            }

            // --- Backtrack ---
            // After returning from the recursive call (or skipping it), remove the
            // prefix component that was added for that level. This "backtracks" the
            // indentation prefix for the next entry at the *current* level.
            current_prefix.truncate(current_prefix.len() - prefix_component.len());
        }
    }

    Ok(())
}

/// # Read and Sort Directory Entries (`read_and_sort_dir_entries`)
///
/// Reads the entries of a given directory path, filters out hidden entries
/// (those starting with a '.'), attempts to get metadata for each entry to
/// determine if it's a directory, and finally sorts the entries.
///
/// ## Sorting Order:
/// 1. Directories first, then files.
/// 2. Within each group (directories/files), sort alphabetically by name.
///
/// ## Error Handling:
/// - Logs warnings and skips entries if reading an entry or its metadata fails.
/// - Returns an error if the initial `fs::read_dir` call fails.
///
/// ## Arguments
///
/// * `dir` - The `&Path` of the directory whose entries should be read and sorted.
///
/// ## Returns
///
/// * `Result<Vec<DirEntry>>` - A vector of `DirEntry` structs, sorted appropriately,
///   or an `Err` if the directory cannot be read.
fn read_and_sort_dir_entries(dir: &Path) -> Result<Vec<DirEntry>> {
    // Vector to hold the processed directory entries.
    let mut collected_entries = Vec::new();

    // Attempt to get an iterator over the directory entries.
    let read_dir_iter = fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory entries from '{}'", dir.display()))?;

    // Process each entry from the iterator.
    for entry_result in read_dir_iter {
        let entry = match entry_result {
            Ok(e) => e, // Successfully read the directory entry.
            Err(e) => {
                // Failed to read a specific entry (e.g., permissions). Log and skip.
                warn!(
                    "Failed to process directory entry in '{}': {}. Skipping.",
                    dir.display(),
                    e
                );
                continue;
            }
        };
        let path = entry.path(); // Get the full path of the entry.
        let name = entry.file_name().to_string_lossy().to_string(); // Get the entry's name.

        // --- Filtering ---
        // Skip hidden files and directories (names starting with '.').
        if name.starts_with('.') {
            debug!("Skipping hidden entry: {}", path.display());
            continue;
        }

        // --- Get Metadata and Determine Type ---
        // Try to get metadata to determine if it's a directory.
        let metadata = match entry.metadata() {
            Ok(md) => md, // Successfully got metadata.
            Err(e) => {
                // Failed to get metadata (e.g., permissions, broken symlink).
                warn!(
                    "Could not get metadata for '{}': {}. Assuming file.",
                    path.display(),
                    e
                );
                // Even if metadata fails, add it as a file so it shows up in the listing.
                collected_entries.push(DirEntry {
                    path,
                    name,
                    is_dir: false, // Assume file on error
                });
                continue; // Skip further processing for this entry.
            }
        };

        // Determine if it's a directory. Note: `is_dir()` follows symlinks.
        // Cycle detection during traversal handles symlink issues.
        let is_dir = metadata.is_dir();

        // Add the processed entry to the collection.
        collected_entries.push(DirEntry { path, name, is_dir });
    }

    // --- Sorting ---
    // Sort the collected entries. The sorting logic is:
    // - If types are the same (both dirs or both files), sort alphabetically by name.
    // - If types differ, directories (`is_dir = true`) come before files (`is_dir = false`).
    collected_entries.sort_by(|a, b| {
        if a.is_dir == b.is_dir {
            // Types are the same, sort by name
            a.name.cmp(&b.name)
        } else if a.is_dir {
            // `a` is a directory, `b` is a file; directories come first.
            std::cmp::Ordering::Less
        } else {
            // `a` is a file, `b` is a directory; files come after.
            std::cmp::Ordering::Greater
        }
    });

    // Return the sorted vector of entries.
    Ok(collected_entries)
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)] // Symlink test is Unix-specific
    use std::os::unix::fs as unix_fs;
    use tempfile::tempdir; // For creating temporary directories for testing

    /// Test basic tree printing functionality to a string.
    /// Creates a simple directory structure and verifies the output string contains
    /// the expected names, connectors, and formatting.
    #[test]
    fn test_print_directory_tree_to_string_basic() -> Result<()> {
        let temp_dir = tempdir()?; // Create a temporary directory
        let root = temp_dir.path();

        // Create a test structure inside the temporary directory:
        // root/
        // ├── Cargo.toml
        // ├── src/
        // │   └── main.rs
        // ├── tests/
        // │   ├── mod.rs
        // │   └── integration.rs
        // ├── .gitignore (hidden)
        // └── .git/ (hidden)
        fs::write(root.join("Cargo.toml"), "[package]")?;
        fs::create_dir(root.join("src"))?;
        fs::write(root.join("src/main.rs"), "fn main() {}")?;
        fs::create_dir(root.join("tests"))?;
        fs::write(root.join("tests/mod.rs"), "// mod")?;
        fs::write(root.join("tests/integration.rs"), "// integration")?;
        fs::write(root.join(".gitignore"), "target/")?; // Hidden file
        fs::create_dir(root.join(".git"))?; // Hidden directory

        // Generate the tree string.
        let tree_string = print_directory_tree_to_string(root, "test-project")?;

        // Print for debugging if test fails.
        println!(
            "--- Tree Output String ---\n{}\n--- End Tree Output ---",
            tree_string
        );

        // --- Assertions ---
        // Check root node name and formatting.
        assert!(tree_string.contains(&format!("{}{}/{}", BOLD_START, "test-project", BOLD_END)));
        // Check directories are present and bolded.
        assert!(tree_string.contains(&format!("{}src{}", BOLD_START, BOLD_END)));
        assert!(tree_string.contains(&format!("{}tests{}", BOLD_START, BOLD_END)));
        // Check files are present (not bolded).
        assert!(tree_string.contains("Cargo.toml"));
        assert!(tree_string.contains("main.rs"));
        assert!(tree_string.contains("integration.rs"));
        assert!(tree_string.contains("mod.rs"));

        // Check hidden files/dirs are skipped.
        assert!(!tree_string.contains(".gitignore"));
        assert!(!tree_string.contains(".git"));

        // Check connectors and indentation based on sorted order (src, tests, Cargo.toml).
        assert!(tree_string.contains(&format!("├── {}{}{}", BOLD_START, "src", BOLD_END))); // Check with bold codes
        assert!(tree_string.contains("│   └── main.rs")); // main.rs is only item in src.
        assert!(tree_string.contains(&format!("├── {}{}{}", BOLD_START, "tests", BOLD_END))); // Check with bold codes
        assert!(tree_string.contains("│   ├── integration.rs")); // integration.rs first in tests.
        assert!(tree_string.contains("│   └── mod.rs")); // mod.rs is last in tests.
        assert!(tree_string.contains("└── Cargo.toml")); // Cargo.toml is the last item at root.

        Ok(())
    }

    /// Test handling of a root path that does not exist.
    #[test]
    fn test_print_directory_tree_nonexistent() {
        let non_existent = PathBuf::from("/path/that/absolutely/does/not/exist");
        let result = print_directory_tree_to_string(&non_existent, "nonexistent");
        // Expect an error.
        assert!(result.is_err());
        // Check if the error message indicates the path doesn't exist.
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    /// Test handling when the provided root path is a file, not a directory.
    #[test]
    fn test_print_directory_tree_file_as_root() -> Result<()> {
        let temp_dir = tempdir()?;
        let file_path = temp_dir.path().join("a_file.txt");
        fs::write(&file_path, "content")?; // Create a file

        let result = print_directory_tree_to_string(&file_path, "a_file");
        // Expect an error.
        assert!(result.is_err());
        // Check if the error message indicates it's not a directory.
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("is not a directory"));
        Ok(())
    }

    /// Test cycle detection using symbolic links (Unix-specific test).
    #[cfg(unix)] // Only run this test on Unix-like systems that support symlinks easily.
    #[test]
    fn test_print_directory_tree_symlink_cycle() -> Result<()> {
        let temp_dir = tempdir()?;
        let root = temp_dir.path();

        // Create structure: root/dir1/link_to_root -> ../../
        let dir1 = root.join("dir1");
        fs::create_dir(&dir1)?;
        let link_path = dir1.join("link_to_root");
        // Create a relative symlink pointing back to the root directory's parent's parent (effectively root).
        let link_target = PathBuf::from("../"); // Link relative to dir1 points to root

        // Create the symbolic link.
        unix_fs::symlink(&link_target, &link_path).with_context(|| {
            format!(
                "Failed to create symlink from {:?} to {}",
                link_target,
                link_path.display()
            )
        })?;

        println!("--- Testing Symlink Cycle ---");
        // Generate the tree string.
        let tree_string = print_directory_tree_to_string(root, "cycle-test")?;
        println!("{}\n--- End Symlink Cycle Test ---", tree_string);

        // --- Assertions ---
        // Check that dir1 is listed (bolded).
        assert!(tree_string.contains(&format!("{}dir1{}", BOLD_START, BOLD_END)));
        // The symlink itself should be listed as an entry within dir1 (and bolded as it points to a dir).
        // It should be the last entry in dir1.
        assert!(tree_string.contains("└── link_to_root"));
        Ok(())
    }

    /// Test the sorting order: directories first, then files, alphabetically within each group.
    #[test]
    fn test_directory_sorting() -> Result<()> {
        let temp_dir = tempdir()?;
        let root = temp_dir.path();

        // Create files and directories out of alphabetical order.
        fs::write(root.join("file_z.txt"), "")?;
        fs::create_dir(root.join("dir_c"))?;
        fs::write(root.join("file_a.txt"), "")?;
        fs::create_dir(root.join("dir_b"))?;
        fs::create_dir(root.join("dir_a"))?;

        // Read and sort the entries using the function under test.
        let entries = read_and_sort_dir_entries(root)?;

        // --- Assertions ---
        // Check the total number of entries (excluding hidden).
        assert_eq!(entries.len(), 5);
        // Verify the sorted order.
        assert_eq!(entries[0].name, "dir_a"); // Dirs first, alphabetically.
        assert!(entries[0].is_dir);
        assert_eq!(entries[1].name, "dir_b");
        assert!(entries[1].is_dir);
        assert_eq!(entries[2].name, "dir_c");
        assert!(entries[2].is_dir);
        assert_eq!(entries[3].name, "file_a.txt"); // Files after, alphabetically.
        assert!(!entries[3].is_dir);
        assert_eq!(entries[4].name, "file_z.txt");
        assert!(!entries[4].is_dir);

        Ok(())
    }

    /// Test printing the tree of an empty directory.
    #[test]
    fn test_print_directory_tree_empty() -> Result<()> {
        let temp_dir = tempdir()?;
        let root = temp_dir.path(); // An empty directory

        // Generate the tree string for the empty directory.
        let tree_string = print_directory_tree_to_string(root, "empty-project")?;
        println!(
            "--- Empty Tree Output ---\n{}\n--- End Empty Tree Output ---",
            tree_string
        );

        // --- Assertion ---
        // The output should only contain the root directory name, formatted and trimmed.
        assert_eq!(
            tree_string.trim(), // Trim whitespace/newlines
            // Expect just the bolded root name with a slash
            format!("{}{}/{}", BOLD_START, "empty-project", BOLD_END)
        );

        Ok(())
    }
}
