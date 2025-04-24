//! # DevRS TAR Archive Operations (`common::archive::tar`)
//!
//! File: cli/src/common/archive/tar.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module provides functionality specifically for creating TAR archives,
//! with a focus on generating gzipped tarballs (`.tar.gz`). Its primary use
//! within DevRS is to create the build context that needs to be sent to the
//! Docker daemon when building Docker images (`devrs container build`, `devrs env build`).
//!
//! ## Architecture
//!
//! The module leverages the `tar` crate for building the archive structure and
//! the `flate2` crate for Gzip compression.
//!
//! - It reads the contents of a specified directory recursively.
//! - Files and directories are added to the archive with paths relative to the root
//!   of the specified directory.
//! - The entire archive is compressed using Gzip and returned as a byte vector.
//!
//! ## Usage
//!
//! The main function `create_context_tar` is used to generate the archive data in memory.
//!
//! ```rust
//! use crate::common::archive::tar; // Import the tar module
//! use crate::core::error::Result; // Use standard Result
//! use std::path::Path;
//! # use std::{fs, env};
//! # use tempfile::tempdir;
//!
//! # fn main() -> Result<()> {
//! # let temp_dir = tempdir()?;
//! # fs::write(temp_dir.path().join("file.txt"), "content")?;
//! # let context_path_obj = temp_dir.path();
//! let context_path = Path::new(context_path_obj); // Path to the directory to archive
//!
//! // Create a gzipped tar archive in memory
//! let tar_gz_bytes: Vec<u8> = tar::create_context_tar(context_path)?;
//!
//! println!("Generated tar.gz archive with size: {} bytes", tar_gz_bytes.len());
//!
//! // These bytes can now be used, e.g., uploaded as a Docker build context.
//! // (Conceptual Docker interaction)
//! // docker_client.build_image(options, None, Some(tar_gz_bytes.into())).await?;
//! # Ok(())
//! # }
//! ```
//!
use crate::core::error::Result; // Use the standard Result type from the core module
use anyhow::Context; // For adding contextual information to errors
use std::path::Path; // Filesystem path type

/// # Create Gzipped TAR Build Context (`create_context_tar`)
///
/// Creates a gzipped TAR archive (`.tar.gz`) in memory containing the contents
/// of the specified `context_path` directory.
///
/// This function is primarily designed to generate the build context required by
/// the Docker daemon for building images. It recursively includes all files and
/// directories within `context_path`, preserving the relative structure. Hidden files
/// or special filesystem objects might be included based on the `tar` crate's default behavior.
///
/// ## Arguments
///
/// * `context_path` - A `&Path` reference to the directory whose contents should be archived.
///                    This directory *must* exist.
///
/// ## Returns
///
/// * `Result<Vec<u8>>` - A `Result` containing the raw bytes (`Vec<u8>`) of the
///   generated `.tar.gz` archive if successful.
///
/// ## Errors
///
/// Returns an `Err` if:
/// - The `context_path` directory cannot be read or accessed.
/// - Any file or subdirectory within `context_path` cannot be added to the archive (e.g., permissions issues).
/// - Finishing the TAR archive structure fails.
/// - Finishing the Gzip compression stream fails.
pub fn create_context_tar(context_path: &Path) -> Result<Vec<u8>> {
    // Create a vector to hold the compressed archive bytes in memory.
    let mut tar_gz_bytes = Vec::new();
    // Wrap the byte vector with a Gzip encoder using default compression level.
    let enc = flate2::write::GzEncoder::new(&mut tar_gz_bytes, flate2::Compression::default());
    // Create a TAR archive builder that writes to the Gzip encoder.
    let mut tar_builder = tar::Builder::new(enc);

    // Add the entire contents of the `context_path` directory recursively.
    // The first argument "." specifies that paths within the archive should be relative
    // to the root of the archive itself (e.g., files in `context_path/src` will appear as `src/...` inside the tar).
    tar_builder
        .append_dir_all(".", context_path)
        .with_context(|| { // Add context to potential errors during directory traversal/adding.
            format!(
                "Failed to add directory '{}' contents to the tar archive",
                context_path.display()
            )
        })?;

    // Finalize the TAR archive structure. This writes necessary closing records.
    // Need to get the inner writer (the Gzip encoder) back from the builder.
    let encoder = tar_builder
        .into_inner()
        .context("Failed to finalize tar archive structure")?;

    // Finalize the Gzip compression stream. This writes compression footers and ensures all data is flushed.
    encoder
        .finish()
        .context("Failed to finish gzip compression stream")?;

    // Return the byte vector containing the complete compressed archive.
    Ok(tar_gz_bytes)
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    // Unit tests remain unchanged as the code logic was not modified.
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use flate2::read::GzDecoder;
    use tar::Archive;

    #[test]
    fn test_create_context_tar_basic() -> Result<()> {
        let temp_dir = tempdir()?;
        let dir_path = temp_dir.path();
        fs::write(dir_path.join("file1.txt"), "hello")?;
        fs::create_dir(dir_path.join("subdir"))?;
        fs::write(dir_path.join("subdir/file2.txt"), "world")?;
        let tar_data = create_context_tar(dir_path)?;
        assert!(!tar_data.is_empty());
        let gz_decoder = GzDecoder::new(tar_data.as_slice());
        let mut tar_archive = Archive::new(gz_decoder);
        let mut found_files = std::collections::HashSet::new();
        for entry_result in tar_archive.entries()? {
            let entry = entry_result?;
            let path = entry.path()?.to_path_buf();
            println!("Found in tar: {:?}", path);
            found_files.insert(path.to_string_lossy().to_string().replace('\\', "/"));
        }
        assert!(found_files.contains("file1.txt"));
        assert!(found_files.contains("subdir/file2.txt"));
        assert!(found_files.contains("subdir"));
        Ok(())
    }
}
