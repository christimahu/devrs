//! # DevRS Project Type Detector
//!
//! File: cli/src/commands/blueprint/utils/project_detector.rs
//! Author: Christi Mahu
//! Repository: https://github.com/christimahu/devrs
//!
//! **DISCLAIMER:** This repository is in the early phases of being rewritten
//! and is not suitable for production development yet.
//!
//! ## Overview
//!
//! This module provides functionality to inspect the contents of a blueprint
//! directory (looking for specific marker files like `Cargo.toml`, `package.json`,
//! `CMakeLists.txt`, etc.) to determine the likely programming language or
//! framework and the associated build system or package manager.
//!
//! ## Architecture
//!
//! The detection follows a priority-based approach:
//!
//! 1. Look for primary marker files (Cargo.toml, go.mod, package.json, etc.)
//! 2. If a marker is found, return the corresponding project type and build system
//! 3. If no markers are found, fall back to scanning for file extensions
//! 4. If still undetermined, return "Unknown" for both fields
//!
//! ## Usage
//!
//! This is primarily used by the `blueprint info` command to provide details
//! about blueprint templates:
//!
//! ```rust
//! use crate::commands::blueprint::utils::project_detector;
//!
//! // Detect project type from blueprint directory
//! let project_info = project_detector::detect_project_type(&blueprint_path);
//!
//! // Use the information
//! println!("Project Type: {}", project_info.project_type);
//! println!("Build System: {}", project_info.build_system);
//! ```
//!
//! The detector also examines package.json contents to determine specific
//! JavaScript frameworks and build tools when relevant.
//!
use std::{fs, path::Path};
use tracing::debug;

/// # Project Information Struct (`ProjectInfo`)
///
/// This struct holds the results of the project type detection process.
/// It stores the identified primary language/framework and the corresponding
/// build system or package manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInfo {
    /// The primary programming language or framework detected (e.g., "Rust", "Go", "Node.js").
    /// Defaults to "Unknown" if detection fails.
    pub project_type: String,
    /// The build system or package manager identified (e.g., "Cargo", "Go Modules", "npm/yarn").
    /// Defaults to "Unknown" if detection fails or is ambiguous (e.g., when detecting by extension only).
    pub build_system: String,
}

/// # Default Project Information
///
/// Provides a default instance of `ProjectInfo` where both the project type
/// and build system are set to "Unknown". This is used as the return value
/// when `detect_project_type` cannot confidently identify the blueprint's nature.
impl Default for ProjectInfo {
    /// Creates a default ProjectInfo indicating "Unknown" type and system.
    fn default() -> Self {
        ProjectInfo {
            project_type: "Unknown".to_string(),
            build_system: "Unknown".to_string(),
        }
    }
}

/// # Detect Project Type (`detect_project_type`)
///
/// Analyzes the contents of the provided `blueprint_path` directory to determine
/// the likely project type (language/framework) and its associated build system.
///
/// ## Detection Strategy
///
/// The detection process follows these steps in order:
///
/// 1.  **Marker File Check:** It first looks for specific, unambiguous marker files
///     that strongly indicate a particular technology stack (e.g., `Cargo.toml` for Rust,
///     `go.mod` for Go, `package.json` for JavaScript/Node.js, `CMakeLists.txt` for C/C++, etc.).
///     The checks are ordered to prioritize more specific detections (e.g., checking for Rust before C/C++).
/// 2.  **Extension Fallback:** If no primary marker files are found, it scans the files
///     directly within the `blueprint_path` (not recursively) for common source code
///     file extensions (e.g., `.rs`, `.go`, `.py`, `.js`). This is less reliable and
///     typically only identifies the language, not the build system.
/// 3.  **Unknown:** If neither marker files nor common extensions yield a result,
///     it returns the default `ProjectInfo` with "Unknown" values.
///
/// ## Arguments
///
/// * `blueprint_path` - A `&Path` reference to the directory containing the blueprint files.
///
/// ## Returns
///
/// * `ProjectInfo` - A struct containing the detected `project_type` and `build_system` as strings.
pub fn detect_project_type(blueprint_path: &Path) -> ProjectInfo {
    debug!("Detecting project type in: {}", blueprint_path.display());

    // --- Primary Marker File Checks ---
    // The order determines priority if multiple markers could potentially exist.

    // Check for Rust marker file
    if check_rust(blueprint_path) {
        return ProjectInfo {
            project_type: "Rust".to_string(),
            build_system: "Cargo".to_string(),
        };
    }
    // Check for Go marker file
    if check_go(blueprint_path) {
        return ProjectInfo {
            project_type: "Go".to_string(),
            build_system: "Go Modules".to_string(),
        };
    }
    // Check for JavaScript/Node.js marker file (and potentially infer framework/build tool)
    if let Some(js_info) = check_javascript(blueprint_path) {
        return js_info;
    }
    // Check for C/C++ CMake marker file
    if check_cmake(blueprint_path) {
        return ProjectInfo {
            project_type: "C/C++".to_string(),
            build_system: "CMake".to_string(),
        };
    }
    // Check for Java/Kotlin build tool marker files (Maven or Gradle)
    if let Some(java_info) = check_java_kotlin(blueprint_path) {
        return java_info;
    }
    // Check for Python marker files (pyproject, requirements, setup.py)
    if let Some(python_info) = check_python(blueprint_path) {
        return python_info;
    }
    // Check for PHP marker file
    if check_php(blueprint_path) {
        return ProjectInfo {
            project_type: "PHP".to_string(),
            build_system: "Composer".to_string(),
        };
    }
    // Check for Ruby marker file
    if check_ruby(blueprint_path) {
        return ProjectInfo {
            project_type: "Ruby".to_string(),
            build_system: "Bundler".to_string(),
        };
    }

    // --- Fallback: Check File Extensions ---
    // If no specific build/project files were found, try identifying by source file extensions.
    if let Some(fallback_type) = check_extensions(blueprint_path) {
        debug!(
            "Detected type '{}' based on file extensions.",
            fallback_type
        );
        return ProjectInfo {
            project_type: fallback_type,
            // Note: Build system cannot be reliably determined just from extensions.
            build_system: "Unknown (by extension)".to_string(),
        };
    }

    // --- Default ---
    // If no markers or known extensions were found.
    debug!("Could not determine project type.");
    ProjectInfo::default()
}

// --- Helper functions for specific language/tool checks ---

/// Checks if a specific file exists directly within the base path.
fn path_exists(base: &Path, file_name: &str) -> bool {
    base.join(file_name).exists()
}

/// Checks for the presence of `Cargo.toml`.
fn check_rust(path: &Path) -> bool {
    path_exists(path, "Cargo.toml")
}

/// Checks for the presence of `go.mod`.
fn check_go(path: &Path) -> bool {
    path_exists(path, "go.mod")
}

/// Checks for the presence of `CMakeLists.txt`.
fn check_cmake(path: &Path) -> bool {
    path_exists(path, "CMakeLists.txt")
}

/// Checks for the presence of `composer.json`.
fn check_php(path: &Path) -> bool {
    path_exists(path, "composer.json")
}

/// Checks for the presence of `Gemfile`.
fn check_ruby(path: &Path) -> bool {
    path_exists(path, "Gemfile")
}

/// Checks for `package.json` and attempts to infer details about the JS/TS project.
/// Looks for common framework dependencies and build tools within the file content.
fn check_javascript(path: &Path) -> Option<ProjectInfo> {
    if path_exists(path, "package.json") {
        let mut project_type = "Node.js".to_string(); // Default type if package.json exists
        let mut build_system = "npm/yarn".to_string(); // Default build system

        // Try to read package.json for more specific clues
        if let Ok(content) = fs::read_to_string(path.join("package.json")) {
            let content_lc = content.to_lowercase(); // Use lowercase for case-insensitive checks

            // Check for common frontend framework dependencies
            if content_lc.contains("\"react\"")
                || content_lc.contains("\"vue\"")
                || content_lc.contains("\"angular\"") // Note: Angular might also have angular.json
                || content_lc.contains("\"svelte\"")
            {
                project_type = "Frontend JavaScript/TypeScript".to_string();
            }
            // Check if it's explicitly a TypeScript project (if not already classified as frontend)
            else if content_lc.contains("\"typescript\"") {
                project_type = "TypeScript".to_string();
            }

            // Check for common build tools/bundlers (order might matter if multiple are present)
            if content_lc.contains("\"webpack\"") {
                build_system = "Webpack".to_string();
            } else if content_lc.contains("\"vite\"") {
                build_system = "Vite".to_string();
            } else if content_lc.contains("\"parcel\"") {
                build_system = "Parcel".to_string();
            } else if content_lc.contains("\"rollup\"") {
                build_system = "Rollup".to_string();
            } else if content_lc.contains("\"esbuild\"") {
                build_system = "esbuild".to_string();
            }
            // Add checks for other tools like Gulp, Grunt if needed
        }
        // Return the detected info
        Some(ProjectInfo {
            project_type,
            build_system,
        })
    } else {
        // No package.json found
        None
    }
}

/// Checks for Java/Kotlin marker files (`pom.xml` for Maven, `build.gradle`/`.kts` for Gradle).
fn check_java_kotlin(path: &Path) -> Option<ProjectInfo> {
    if path_exists(path, "pom.xml") {
        Some(ProjectInfo {
            project_type: "Java".to_string(), // Primarily Java, but could be Kotlin
            build_system: "Maven".to_string(),
        })
    } else if path_exists(path, "build.gradle") || path_exists(path, "build.gradle.kts") {
        Some(ProjectInfo {
            project_type: "Java/Kotlin".to_string(), // Common for both
            build_system: "Gradle".to_string(),
        })
    } else {
        None
    }
}

/// Checks for Python marker files (`pyproject.toml`, `requirements.txt`, `setup.py`, `setup.cfg`).
/// Infers the likely build system based on the files found.
fn check_python(path: &Path) -> Option<ProjectInfo> {
    let has_pyproject = path_exists(path, "pyproject.toml");
    let has_requirements = path_exists(path, "requirements.txt");
    let has_setup_py = path_exists(path, "setup.py");
    let has_setup_cfg = path_exists(path, "setup.cfg");

    // If any Python marker file is present
    if has_pyproject || has_requirements || has_setup_py || has_setup_cfg {
        // Determine build system based on which files are present
        let build_system = if has_pyproject {
            // pyproject.toml is the modern standard, often used with pip, poetry, pdm, etc.
            // Further parsing could refine this (e.g., check for [tool.poetry]).
            "pyproject/pip".to_string()
        } else if has_setup_py || has_setup_cfg {
            // setup.py/cfg indicates setuptools, typically used with pip.
            "setuptools/pip".to_string()
        } else {
            // Only requirements.txt found, implies basic pip usage.
            "pip".to_string()
        };
        Some(ProjectInfo {
            project_type: "Python".to_string(),
            build_system,
        })
    } else {
        None
    }
}

/// # Check File Extensions (Fallback)
///
/// Scans the immediate files (non-recursive) within the `path` directory
/// for common source code extensions as a fallback detection method when
/// no primary marker files (like `Cargo.toml`, `package.json`, etc.) are found.
///
/// Note: This method is less reliable than marker file detection and cannot
/// usually determine the build system. It stops and returns the type associated
/// with the *first* recognized extension it encounters.
///
/// ## Arguments
///
/// * `path` - The directory path to scan for file extensions.
///
/// ## Returns
///
/// * `Option<String>` - The name of the detected language (e.g., "Rust", "Python")
///     if a known extension is found, otherwise `None`.
fn check_extensions(path: &Path) -> Option<String> {
    // Attempt to read directory entries
    let entries = match fs::read_dir(path) {
        Ok(iter) => iter,
        Err(e) => {
            // Log if directory reading fails, but don't error out the whole detection
            debug!(
                "Could not read directory {} for extension check: {}",
                path.display(),
                e
            );
            return None; // Cannot check extensions if directory is unreadable
        }
    };

    // Iterate through directory entries
    for entry_result in entries {
        if let Ok(entry) = entry_result {
            // Get the file extension as a string slice, if possible
            if let Some(ext) = entry.path().extension().and_then(|os| os.to_str()) {
                // Match against known common extensions
                // Return the type as soon as the first match is found.
                match ext {
                    "rs" => return Some("Rust".to_string()),
                    "go" => return Some("Go".to_string()),
                    "py" => return Some("Python".to_string()),
                    "js" | "mjs" | "cjs" => return Some("JavaScript".to_string()),
                    "ts" | "tsx" => return Some("TypeScript".to_string()),
                    "java" => return Some("Java".to_string()),
                    "kt" | "kts" => return Some("Kotlin".to_string()),
                    "c" | "h" => return Some("C".to_string()),
                    "cpp" | "hpp" | "cxx" | "hxx" => return Some("C++".to_string()),
                    "rb" => return Some("Ruby".to_string()),
                    "php" => return Some("PHP".to_string()),
                    // Add other extensions as needed (e.g., "swift", "scala")
                    _ => {} // Ignore unrecognized extensions
                }
            }
        }
        // Ignore entries that cause errors (e.g., permission issues)
    }
    None // No recognized extension found among the files scanned
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // Helper to create a mock blueprint directory with specified files
    fn create_mock_bp(files: &[&str]) -> tempfile::TempDir {
        let temp_dir = tempdir().unwrap();
        for file in files {
            let file_path = temp_dir.path().join(file);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(file_path, "").unwrap(); // Create empty file
        }
        temp_dir
    }

    // Helper to create a mock blueprint directory with file content
    fn create_mock_bp_with_content(files: &[(&str, &str)]) -> tempfile::TempDir {
        let temp_dir = tempdir().unwrap();
        for (file, content) in files {
            let file_path = temp_dir.path().join(file);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(file_path, content).unwrap();
        }
        temp_dir
    }

    #[test]
    fn test_detect_rust() {
        let bp = create_mock_bp(&["Cargo.toml", "src/main.rs"]);
        let info = detect_project_type(bp.path());
        assert_eq!(
            info,
            ProjectInfo {
                project_type: "Rust".to_string(),
                build_system: "Cargo".to_string()
            }
        );
    }

    #[test]
    fn test_detect_go() {
        let bp = create_mock_bp(&["go.mod", "main.go"]);
        let info = detect_project_type(bp.path());
        assert_eq!(
            info,
            ProjectInfo {
                project_type: "Go".to_string(),
                build_system: "Go Modules".to_string()
            }
        );
    }

    #[test]
    fn test_detect_nodejs() {
        let bp = create_mock_bp_with_content(&[(
            "package.json",
            r#"{"dependencies": {"express": "4.x"}}"#,
        )]);
        let info = detect_project_type(bp.path());
        assert_eq!(
            info,
            ProjectInfo {
                project_type: "Node.js".to_string(),
                build_system: "npm/yarn".to_string() // Default JS build system
            }
        );
    }

    #[test]
    fn test_detect_frontend_react() {
        let bp = create_mock_bp_with_content(&[(
            "package.json",
            r#"{"dependencies": {"react": "18.x"}}"#,
        )]);
        let info = detect_project_type(bp.path());
        assert_eq!(
            info,
            ProjectInfo {
                // Type correctly identified as Frontend due to "react" dependency
                project_type: "Frontend JavaScript/TypeScript".to_string(),
                build_system: "npm/yarn".to_string() // Default build system
            }
        );
    }

    #[test]
    fn test_detect_frontend_vite() {
        let bp = create_mock_bp_with_content(&[(
            "package.json",
            r#"{"devDependencies": {"vite": "3.x"}}"#,
        )]);
        let info = detect_project_type(bp.path());
        // Type remains Node.js/TS (no specific framework), but build system is Vite
        assert!(info.project_type.contains("Node.js") || info.project_type.contains("TypeScript"));
        assert_eq!(info.build_system, "Vite".to_string());
    }

    #[test]
    fn test_detect_python_pip() {
        let bp = create_mock_bp(&["requirements.txt", "app.py"]);
        let info = detect_project_type(bp.path());
        assert_eq!(
            info,
            ProjectInfo {
                project_type: "Python".to_string(),
                build_system: "pip".to_string() // Inferred from requirements.txt only
            }
        );
    }

    #[test]
    fn test_detect_python_pyproject() {
        let bp = create_mock_bp(&["pyproject.toml", "src/main.py"]);
        let info = detect_project_type(bp.path());
        assert_eq!(
            info,
            ProjectInfo {
                project_type: "Python".to_string(),
                // Inferred from pyproject.toml presence
                build_system: "pyproject/pip".to_string()
            }
        );
    }

    #[test]
    fn test_detect_cmake() {
        let bp = create_mock_bp(&["CMakeLists.txt", "src/main.cpp"]);
        let info = detect_project_type(bp.path());
        assert_eq!(
            info,
            ProjectInfo {
                project_type: "C/C++".to_string(),
                build_system: "CMake".to_string()
            }
        );
    }

    #[test]
    fn test_detect_maven() {
        let bp = create_mock_bp(&["pom.xml", "src/main/java/App.java"]);
        let info = detect_project_type(bp.path());
        assert_eq!(
            info,
            ProjectInfo {
                project_type: "Java".to_string(), // pom.xml implies Java/Maven
                build_system: "Maven".to_string()
            }
        );
    }

    #[test]
    fn test_detect_gradle() {
        let bp = create_mock_bp(&["build.gradle", "src/main/kotlin/App.kt"]);
        let info = detect_project_type(bp.path());
        assert_eq!(
            info,
            ProjectInfo {
                project_type: "Java/Kotlin".to_string(), // gradle implies Java or Kotlin
                build_system: "Gradle".to_string()
            }
        );
    }

    #[test]
    fn test_detect_fallback_extension() {
        let bp = create_mock_bp(&["main.rb"]); // Only a Ruby file, no Gemfile marker
        let info = detect_project_type(bp.path());
        assert_eq!(
            info,
            ProjectInfo {
                project_type: "Ruby".to_string(), // Detected via .rb extension
                build_system: "Unknown (by extension)".to_string() // Build system unknown
            }
        );
    }

    #[test]
    fn test_detect_unknown() {
        // Contains no recognizable marker files or source extensions
        let bp = create_mock_bp(&["README.md", "config.txt", "data.xml"]);
        let info = detect_project_type(bp.path());
        // Should return the default "Unknown" values
        assert_eq!(info, ProjectInfo::default());
    }
}
