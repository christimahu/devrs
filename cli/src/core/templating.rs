//! # DevRS Template System
//!
//! File: cli/src/core/templating.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten 
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module implements the template rendering system used by the blueprint
//! functionality to create new projects from templates. It handles directory
//! traversal, variable substitution, and file copying/creation.
//!
//! ## Architecture
//!
//! The template system uses the Tera templating engine and follows these steps:
//! 1. Recursively scan the source directory (blueprint template)
//! 2. For each file:
//!    - If it's a template file (by extension), render it with variables
//!    - Otherwise, copy it directly
//! 3. Create the target directory structure mirroring the source
//!
//! Features:
//! - Handles template files with extensions like `.template`, `.tmpl`, `.tera`
//! - Preserves directory structure
//! - Skips hidden files and directories (starting with `.`)
//! - Provides detailed error information
//!
//! ## Examples
//!
//! Using the template system:
//!
//! // Prepare context variables
//! let mut context = HashMap::new();
//! context.insert("project_name".to_string(), "my-awesome-app".to_string());
//! context.insert("author".to_string(), "Christi Mahu".to_string());
//!
//! // Define which extensions are treated as templates
//! let template_extensions = [".template", ".tmpl", ".tera"];
//!
//! // Render the template
//! templating::render_template_directory(
//!     &source_path,
//!     &target_path,
//!     &context,
//!     &template_extensions,
//! )?;
//!
//! The template system is used by the blueprint commands to create new projects
//! with customized filenames, content, and structure based on the blueprint template.
//!
use crate::core::error::{DevrsError, Result}; // Use error from the same core module
use anyhow::{anyhow, Context};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tera::Tera;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

// --- Function `render_template_directory` remains the same ---
// ... (Paste the function from the previous response here) ...
pub fn render_template_directory(
    source_dir: &Path,
    target_dir: &Path,
    context_map: &HashMap<String, String>, // Simple context for now
    template_extensions: &[&str],
) -> Result<()> {
    info!(
        "Starting template processing from '{}' to '{}'",
        source_dir.display(),
        target_dir.display()
    );
    fs::create_dir_all(target_dir).with_context(|| {
        format!(
            "Failed to create target directory '{}'",
            target_dir.display()
        )
    })?;

    let tera_context = tera::Context::from_serialize(context_map).map_err(|e| {
        anyhow!(DevrsError::Template { source: e })
            .context("Failed to create Tera context from map")
    })?;

    for entry_result in WalkDir::new(source_dir) {
        let entry = match entry_result {
            Ok(e) => e,
            Err(e) => {
                warn!(
                    "Failed to access entry during walk in '{}': {}",
                    source_dir.display(),
                    e
                );
                continue;
            }
        };
        let src_path = entry.path();
        let relative_path = match src_path.strip_prefix(source_dir) {
            Ok(p) => p,
            Err(_) => {
                warn!(
                    "Could not determine relative path for '{}' based on '{}'",
                    src_path.display(),
                    source_dir.display()
                );
                continue;
            }
        };
        let target_path = target_dir.join(relative_path);

        if relative_path
            .components()
            .any(|comp| comp.as_os_str().to_string_lossy().starts_with('.'))
        {
            debug!("Skipping hidden path: {}", src_path.display());
            continue;
        }

        if src_path.is_dir() {
            fs::create_dir_all(&target_path).with_context(|| {
                format!(
                    "Failed to create target subdirectory '{}'",
                    target_path.display()
                )
            })?;
            debug!("Created directory: {}", target_path.display());
        } else if src_path.is_file() {
            let file_name = match src_path.file_name() {
                Some(name) => name.to_string_lossy(),
                None => {
                    warn!("Skipping file without a name: {}", src_path.display());
                    continue;
                }
            };
            let matching_ext = template_extensions
                .iter()
                .find(|&&ext| file_name.to_lowercase().ends_with(&ext.to_lowercase()));

            if let Some(ext_to_remove) = matching_ext {
                debug!("Rendering template file: {}", src_path.display());
                let template_content = fs::read_to_string(src_path).with_context(|| {
                    format!("Failed to read template file '{}'", src_path.display())
                })?;
                let rendered_content = Tera::one_off(&template_content, &tera_context, true)
                    .map_err(|e| {
                        anyhow!(DevrsError::Template { source: e }).context(format!(
                            "Tera rendering failed for template file '{}'",
                            src_path.display()
                        ))
                    })?;
                let final_target_path = if file_name.len() >= ext_to_remove.len()
                    && file_name
                        .to_lowercase()
                        .ends_with(&ext_to_remove.to_lowercase())
                {
                    target_path.with_file_name(&file_name[..file_name.len() - ext_to_remove.len()])
                } else {
                    warn!("Could not accurately strip extension '{}' from '{}', using original target path.", ext_to_remove, target_path.display());
                    target_path.clone()
                };
                fs::write(&final_target_path, rendered_content).with_context(|| {
                    format!(
                        "Failed to write rendered file '{}'",
                        final_target_path.display()
                    )
                })?;
                info!(
                    "Rendered template '{}' to '{}'",
                    src_path.display(),
                    final_target_path.display()
                );
            } else {
                fs::copy(src_path, &target_path).with_context(|| {
                    format!(
                        "Failed to copy file '{}' to '{}'",
                        src_path.display(),
                        target_path.display()
                    )
                })?;
                debug!(
                    "Copied file '{}' to '{}'",
                    src_path.display(),
                    target_path.display()
                );
            }
        } else {
            warn!(
                "Skipping unsupported file system entry type at '{}'",
                src_path.display()
            );
        }
    }
    info!("Template processing completed successfully.");
    Ok(())
}

// --- Unit Tests (Templating tests) remain the same ---
// ... (Paste the tests from the previous response here) ...
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn test_render_and_copy() -> Result<()> {
        let source = tempdir().unwrap();
        let target = tempdir().unwrap();
        let mut context = HashMap::new();
        context.insert("project_name".to_string(), "MyAwesomeApp".to_string());
        context.insert("version".to_string(), "0.1.0".to_string());

        create_file(
            &source.path().join("README.md.template"),
            "# {{ project_name }}\nVersion: {{ version }}",
        );
        create_file(&source.path().join("src/main.rs"), "fn main() {}");
        create_file(&source.path().join("config/settings.toml"), "# Basic");
        create_file(&source.path().join(".gitignore"), "target/\n*.log");
        fs::create_dir_all(source.path().join(".hidden_dir")).unwrap();
        create_file(&source.path().join(".hidden_dir/secret.txt"), "secret");

        let result = render_template_directory(
            source.path(),
            target.path(),
            &context,
            &[".template", ".tera"],
        );
        assert!(result.is_ok());

        let rendered_readme_path = target.path().join("README.md");
        assert!(rendered_readme_path.exists());
        let readme_content = fs::read_to_string(rendered_readme_path)?;
        assert!(readme_content.contains("# MyAwesomeApp"));
        assert!(readme_content.contains("Version: 0.1.0"));
        assert!(target.path().join("src/main.rs").exists());
        assert!(target.path().join("config/settings.toml").exists());
        assert!(!target.path().join(".gitignore").exists());
        assert!(!target.path().join(".hidden_dir").exists());
        assert!(target.path().join("src").is_dir());
        assert!(target.path().join("config").is_dir());

        Ok(())
    }

    #[test]
    fn test_render_empty_dir() -> Result<()> {
        let source = tempdir().unwrap();
        let target = tempdir().unwrap();
        let context = HashMap::new();
        let result =
            render_template_directory(source.path(), target.path(), &context, &[".template"]);
        assert!(result.is_ok());
        assert!(target.path().exists());
        assert_eq!(fs::read_dir(target.path())?.count(), 0);
        Ok(())
    }

    #[test]
    fn test_render_invalid_template_syntax() -> Result<()> {
        let source = tempdir().unwrap();
        let target = tempdir().unwrap();
        let mut context = HashMap::new();
        context.insert("name".to_string(), "test".to_string());
        create_file(&source.path().join("invalid.tera"), "Hello {{ name"); // Invalid syntax

        let result = render_template_directory(source.path(), target.path(), &context, &[".tera"]);
        assert!(result.is_err());
        let error_string = result.unwrap_err().to_string();
        assert!(error_string.contains("Tera rendering failed"));
        assert!(error_string.contains("invalid.tera"));
        assert!(target.path().exists());
        assert!(!target.path().join("invalid").exists());
        Ok(())
    }

    #[test]
    fn test_render_case_insensitive_extension() -> Result<()> {
        let source = tempdir().unwrap();
        let target = tempdir().unwrap();
        let mut context = HashMap::new();
        context.insert("name".to_string(), "MixedCase".to_string());
        create_file(&source.path().join("UPPER.TEMPLATE"), "Hello {{ name }}"); // Uppercase extension

        let result =
            render_template_directory(source.path(), target.path(), &context, &[".template"]);
        assert!(result.is_ok());
        let rendered_path = target.path().join("UPPER");
        assert!(rendered_path.exists());
        let content = fs::read_to_string(rendered_path)?;
        assert_eq!(content, "Hello MixedCase");
        Ok(())
    }
}
