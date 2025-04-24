//! # DevRS HTTP Server Utilities
//!
//! File: cli/src/commands/srv/utils.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module provides utility functions for the HTTP server implementation.
//! It includes helper functions for:
//! - Directory content logging for debugging
//! - Local IP address detection for network URL display
//! - File size and modification time formatting
//!
//! ## Architecture
//!
//! The utilities are designed to support the server_logic module with
//! helper functions that aren't directly part of the server logic but
//! enhance the user experience. They handle various tasks:
//! - Displaying diagnostic information about served directories
//! - Finding the local network IP for external access
//! - Formatting file information for human readability
//!
//! ## Examples
//!
//! Example usage within the server logic:
//!
//! ```rust
//! // Log directory contents for debugging
//! utils::log_directory_contents(&config.directory);
//!
//! // Get local IP for network URL
//! let local_ip = utils::get_local_ip();
//! println!("Network URL: http://{}:{}", local_ip, port);
//!
//! // Format a file size for display
//! let size_str = utils::human_readable_size(metadata.len());
//! println!("File size: {}", size_str);
//! ```
//!
//! These utilities help provide a more user-friendly and informative
//! interface for the HTTP server, making it easier to use for local
//! development.
//!
use std::path::PathBuf;
use tracing::{info, warn};

/// # Log Directory Contents (`log_directory_contents`)
///
/// Reads and logs the entries in the specified directory path at the INFO level.
/// It indicates whether each entry is a file or a directory. If reading the directory
/// or fetching metadata for an entry fails, a warning is logged.
///
/// ## Arguments
///
/// * `path`: A reference to the `PathBuf` of the directory to log.
pub fn log_directory_contents(path: &PathBuf) {
    info!("Directory contents for {}:", path.display());

    // Attempt to read the directory.
    match std::fs::read_dir(path) {
        Ok(entries) => {
            let mut entry_count = 0;

            // Process each entry, skipping entries that couldn't be read.
            for entry in entries.filter_map(Result::ok) {
                entry_count += 1;

                // Try getting metadata to determine type (file/dir).
                if let Ok(metadata) = entry.metadata() {
                    let file_type = if metadata.is_dir() { "DIR " } else { "FILE" };
                    info!(
                        "  - {} : {}",
                        file_type,
                        entry.file_name().to_string_lossy() // Log type and name.
                    );
                } else {
                    // Log warning if metadata read fails.
                    warn!(
                        "  - Could not read metadata for: {}",
                        entry.path().display()
                    );
                }
            }

            // Log if the directory was empty.
            if entry_count == 0 {
                info!("  (Empty directory)");
            }
        }
        Err(e) => {
            // Log warning if reading the directory itself failed.
            warn!(
                "Could not read directory contents for '{}': {}",
                path.display(),
                e
            );
        }
    }
}

/// # Get Local IP Address (`get_local_ip`)
///
/// Attempts to find a non-loopback local network IP address by executing a series of
/// common shell commands for different operating systems (macOS, Linux). It returns the
/// output of the first successful command that yields a non-empty, non-localhost IP.
/// If all commands fail or yield no suitable IP, it logs a warning and returns "localhost".
///
/// Note: Relies on external shell commands (`ipconfig`, `ip addr`, `ifconfig`) being available and working.
///
/// ## Returns
///
/// * `String`: The detected IP address string, or "localhost" as a fallback.
pub fn get_local_ip() -> String {
    info!("Attempting to detect local network IP address");

    // Platform-specific commands to retrieve IP addresses.
    let commands = [
        // macOS / BSD command.
        "ipconfig getifaddr en0",
        // macOS / BSD fallback interface.
        "ipconfig getifaddr en1",
        // Linux command using modern `ip` tool.
        "ip addr show | grep 'inet ' | grep -v '127.0.0.1' | head -n 1 | awk '{print $2}' | cut -d/ -f1",
        // Linux command using older `ifconfig` tool.
        "ifconfig | grep 'inet ' | grep -v '127.0.0.1' | head -n 1 | awk '{print $2}'"
    ];

    // Try each command until one succeeds and returns a valid IP.
    for cmd_str in commands {
        match std::process::Command::new("sh")
            .arg("-c") // Execute command string via shell.
            .arg(cmd_str)
            .output() // Capture stdout/stderr/status.
        {
            Ok(output) if output.status.success() => {
                // Command succeeded, process output.
                let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
                // Check if output is a usable IP address.
                if !ip.is_empty() && ip != "localhost" {
                    info!("Found local IP: {}", ip);
                    return ip; // Return the found IP.
                }
            }
            Ok(output) => {
                // Command ran but failed or produced no output. Log for debugging.
                warn!(
                    "Command '{}' failed or returned empty: status={:?}, stderr={}",
                    cmd_str,
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Err(e) => {
                 // Command failed to execute at all.
                warn!("Error executing command '{}': {}", cmd_str, e);
            }
        }
    }

    // If no command yielded a usable IP, return the default fallback.
    warn!("Could not detect local network IP, falling back to 'localhost'");
    "localhost".to_string()
}

/// # Format File Size (`human_readable_size`)
///
/// Converts a file size given in bytes (u64) into a human-readable string
/// representation using appropriate units (B, KB, MB, GB, TB, PB).
/// Displays bytes without decimals and larger units with one decimal place.
///
/// ## Arguments
///
/// * `size`: The file size in bytes.
///
/// ## Returns
///
/// * `String`: The formatted file size (e.g., "123 B", "1.2 KB", "1.1 GB").
#[allow(dead_code)] // Not currently used, but useful for future enhancements
pub fn human_readable_size(size: u64) -> String {
    // Define the units for sizing.
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

    // Handle the base case of zero bytes.
    if size == 0 {
        return "0 B".to_string();
    }

    // Calculate the appropriate unit exponent based on powers of 1024.
    let base = 1024_f64;
    let exponent = (size as f64).ln() / base.ln();
    let exponent = exponent.floor() as usize;

    // Select the unit string, ensuring it doesn't exceed the defined units.
    let unit = UNITS[exponent.min(UNITS.len() - 1)];
    // Calculate the size relative to the chosen unit.
    let size = size as f64 / base.powi(exponent as i32);

    // Format the output string.
    if exponent == 0 {
        format!("{} {}", size, unit) // Bytes: Show without decimal places.
    } else {
        format!("{:.1} {}", size, unit) // KB and larger: Show with one decimal place.
    }
}

/// # Format Modification Time (`format_modification_time`)
///
/// Formats a `std::time::SystemTime` into a human-readable date and time string
/// in the format "YYYY-MM-DD HH:MM:SS" (UTC).
///
/// ## Arguments
///
/// * `time`: The `SystemTime` representing the file's modification time.
///
/// ## Returns
///
/// * `String`: The formatted date/time string, or "Unknown" if the time cannot be converted or formatted.
#[allow(dead_code)] // Not currently used, but useful for future enhancements
pub fn format_modification_time(time: std::time::SystemTime) -> String {
    // Convert SystemTime to chrono::DateTime<Utc>.
    let datetime = match time.duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => {
            // Attempt conversion to UTC DateTime.
            chrono::DateTime::<chrono::Utc>::from_timestamp(
                duration.as_secs() as i64,
                duration.subsec_nanos(),
            )
        }
        Err(_) => return "Unknown".to_string(), // Handle error during duration calculation (e.g., time before epoch).
    };

    // Ensure the DateTime conversion was successful.
    let datetime = match datetime {
        Some(dt) => dt,
        None => return "Unknown".to_string(), // Handle potential failure in from_timestamp.
    };

    // Format the DateTime into the desired string format.
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

// --- Unit Tests ---

/// # Unit Tests for Server Utilities
///
/// Contains unit tests for the helper functions in the `srv::utils` module.
#[cfg(test)]
mod tests {
    use super::*; // Import items from the parent module (utils.rs).
    use std::fs;
    use tempfile::TempDir; // Used for creating temporary directories for tests.

    /// Test human-readable size formatting for various inputs.
    #[test]
    fn test_human_readable_size() {
        // Test edge case (zero).
        assert_eq!(human_readable_size(0), "0 B");
        // Test bytes.
        assert_eq!(human_readable_size(123), "123 B");
        // Test kilobytes.
        assert_eq!(human_readable_size(1234), "1.2 KB");
        // Test megabytes.
        assert_eq!(human_readable_size(1234567), "1.2 MB");
        // Test gigabytes.
        assert_eq!(human_readable_size(1234567890), "1.1 GB");
        // Test terabytes.
        assert_eq!(human_readable_size(1234567890000), "1.1 TB");
    }

    /// Test the directory content logging function.
    /// Verifies that the function executes without panicking when given a directory
    /// containing files and subdirectories. Does not assert log output.
    #[test]
    fn test_log_directory_contents() {
        // Setup: Create a temporary directory.
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_path_buf();

        // Setup: Create some files and a subdirectory within the temp directory.
        fs::write(dir_path.join("test1.txt"), "test1").unwrap();
        fs::write(dir_path.join("test2.txt"), "test2").unwrap();
        fs::create_dir(dir_path.join("subdir")).unwrap();

        // Action & Assert: Call the function and ensure it doesn't panic.
        log_directory_contents(&dir_path);
    }

    /// Test the local IP detection function.
    /// Verifies that the function returns either "localhost" or a string
    /// containing characters indicative of an IP address format without panicking.
    #[test]
    fn test_get_local_ip() {
        // Action: Execute the IP detection function.
        let ip = get_local_ip();

        // Assert: Check if the returned string is the fallback or looks like an IP.
        assert!(ip == "localhost" || ip.contains('.'));
    }

    /// Test the modification time formatting function.
    /// Verifies that a known `SystemTime` (representing 2021-01-01 UTC) is
    /// formatted correctly into the "YYYY-MM-DD HH:MM:SS" string format.
    #[test]
    fn test_format_modification_time() {
        // Setup: Create a specific SystemTime corresponding to Jan 1, 2021 00:00:00 UTC.
        let unix_time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1609459200);

        // Action: Format the time.
        let formatted = format_modification_time(unix_time);

        // Assert: Check if the formatted string matches the expected output format.
        assert!(formatted.starts_with("2021-01-01 00:00:00"));
    }
}
