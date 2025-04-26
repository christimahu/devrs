# DevRS Architecture Overview

This document outlines the technical architecture and design principles of the DevRS CLI tool, including its core concepts, project structure, and implementation details.

## 1. Core Philosophy

DevRS aims to provide a consistent, powerful, and configurable development environment management system with these key goals:

- **Unified Interface**: Consolidate various development tasks (environment management, containerization, project scaffolding, setup) under a single `devrs` command.
- **Reproducibility**: Leverage Docker to ensure identical development environments across different machines.
- **Flexibility**: Configure mounts, ports, environment variables, and template locations via TOML files, with user configuration managed via a symlink to a repo file.
- **Extensibility**: Follow a modular design to facilitate future additions.
- **Idiomatic Rust**: Utilize Rust best practices for error handling, async programming, and type safety.

## 2. Core Concepts

DevRS organizes functionality around several key concepts:

### 2.1 Core Development Environment (`devrs env`)

Manages a single, consistent Docker container with development tools pre-installed:

- Provides a standardized environment with your complete toolchain
- Based on the `presets/Dockerfile.devrs` file in the DevRS repo
- Operates as a long-running environment used across multiple projects
- Supports building, starting, executing commands, and viewing logs

**Example usage:**

```bash
# Start a shell in the development environment
devrs env shell

# Execute a command within the environment
devrs env exec cargo build

# Check environment status
devrs env status
```

### 2.2 Application Containers (`devrs container`)

Manages application-specific Docker containers:

- Specific to individual applications you're developing
- Based on Dockerfiles in your actual projects
- Used for running your applications, not for development work within the core toolchain
- Supports building images, running containers, managing lifecycle
- Typically short-lived or used for testing/running applications

**Example usage:**

```bash
# Build a container image from the current project
devrs container build --tag myapp:1.0

# Run a container from the built image
devrs container run --image myapp:1.0 --port 8080:80

# View container logs
devrs container logs myapp-instance
```

### 2.3 Project Templates (Blueprints) (`devrs blueprint`)

Manages project templates:

- Templates for creating starter code for various project types
- Supports template variables (using Tera syntax) replaced during creation
- Includes conditional logic and special file handling
- Detects project types (Rust, Go, etc.) and provides appropriate configuration

**Example usage:**

```bash
# List available templates
devrs blueprint list

# Create a new project from a template
devrs blueprint create --lang rust my-project

# View template details
devrs blueprint info rust
```

### 2.4 HTTP Server (`devrs srv`)

Provides a simple HTTP file server:

- Serves static files from a specified directory
- Supports SPA (Single Page Application) mode by routing paths to `index.html`
- Configurable with CORS, port, and other options
- Automatically finds available ports if the requested one is in use
- Shows network links for easy access from other devices

**Example usage:**

```bash
# Serve files from the current directory
devrs srv .

# Specify port and host interface
devrs srv --port 9000 --host 0.0.0.0 ./dist
```

### 2.5 Host Setup (`devrs setup`)

Configures the host system for optimal DevRS usage (must be run from the repo root):

- Checks for required dependencies (`devrs setup deps`)
- Sets up the user `config.toml` by symlinking the standard user config path to `presets/config.toml` (creating it from sample if missing) (`devrs setup config`)
- Provides instructions for shell integration (sourcing `presets/shell_functions`) (`devrs setup integrate`)
- Sets up Neovim configuration (symlinking `presets/init.lua`) and installs the Packer plugin manager (`devrs setup nvim`)
- Runs all steps sequentially by default (`devrs setup` or `devrs setup all`)

**Example usage:**

```bash
# Run all setup tasks (default)
devrs setup

# Provide shell integration instructions only
devrs setup integrate

# Check dependencies only
devrs setup deps
```

## 3. Project Structure

```plaintext
devrs/                       # Root workspace directory
├── .github/                 # GitHub Actions workflows
├── cli/                     # The main CLI application crate
│   ├── src/
│   │   ├── main.rs          # Entry point, Clap setup, command routing
│   │   ├── commands/        # Command group implementations
│   │   │   ├── blueprint/   # Blueprint command implementations
│   │   │   ├── container/   # Container command implementations
│   │   │   ├── env/         # Environment command implementations
│   │   │   ├── setup/       # Setup command implementations
│   │   │   └── srv/         # HTTP server command implementation
│   │   ├── common/          # Shared utility functionality
│   │   │   ├── docker/      # Docker operations
│   │   │   ├── fs/          # Filesystem operations
│   │   │   ├── network/     # Network utilities (Placeholder)
│   │   │   ├── process/     # Process execution
│   │   │   ├── system/      # System-level utilities
│   │   │   ├── ui/          # User interface helpers (Placeholder)
│   │   │   └── archive/     # Archive handling
│   │   └── core/            # Core application infrastructure
│   │       ├── config.rs    # Configuration loading
│   │       ├── error.rs     # Error types
│   │       └── templating.rs# Blueprint templating
├── presets/                 # Configuration presets and sources
│   ├── init.lua             # Neovim configuration source
│   ├── shell_functions      # Shared shell functions source
│   ├── config.toml          # Base config (created by setup if missing)
│   ├── config_sample.toml   # Sample config
│   └── Dockerfile.devrs     # Core development Dockerfile
├── blueprints/              # Project template directories
│   ├── rust/                # Rust project templates
│   ├── go/                  # Go project templates
│   └── cpp/                 # C++ project templates
├── Cargo.toml               # Workspace definition
└── README.md                # Main project README
```

## 4. Implementation Architecture

### 4.1 Command Processing Flow

- **Entry Point (`main.rs`)**
  - Parses command-line arguments using `clap`
  - Sets up logging
  - Routes to appropriate command handler

- **Command Modules (`commands/`)**
  - Each module corresponds to a command group (env, container, etc.)
  - Defines command-specific arguments
  - Implements command logic

- **Command Execution**
  - Load and validate configuration (`core::config`)
  - Interact with Docker (`common::docker`)
  - Process files (`common::fs`)
  - Execute external commands (`std::process::Command` or `tokio::process::Command`)
  - Return structured errors via `core::error`

### 4.2 Core Infrastructure

- **Configuration System (`core/config.rs`)**: Loads from multiple sources, handles path expansion and validation
- **Error Handling (`core/error.rs`)**: Uses `thiserror` and `anyhow`
- **Templating System (`core/templating.rs`)**: Uses `tera` for blueprint rendering

### 4.3 Common Utilities

- **Docker Operations (`common/docker`)**: Abstracts interactions with Bollard
- **Filesystem Utilities (`common/fs`)**: Directory copying, symlinks, etc.
- **Archive Utilities (`common/archive`)**: TAR creation; compression pending
- **Process/System Utilities (`common/process`, `common/system`)**: Minimal implementations
- **Network/UI Utilities (`common/network`, `common/ui`)**: Placeholders

## 5. Development Workflow

1. Clone repository and run `devrs setup`
2. Build environment with `devrs env build`
3. Develop using `devrs env shell`
4. Test with `cargo test --workspace`
5. Build applications with `devrs container build`

## 6. Future Enhancements

- Additional templates
- Enhanced UI/UX
- More robust utilities
- Remote environment support
- Plugin system
- Web dashboard

## 7. Implementation Status

- **Complete**: Configuration, CLI parsing, blueprint creation/listing, HTTP server, error system, setup system, Docker/file utilities
- **Partial**: Container ops, environment ops, blueprint info, setup nvim error handling, minimal process/system helpers
- **Planned**: UI improvements, network utilities, compression utilities

## 8. Design Principles

- Single Responsibility
- Command Isolation
- Clean Configuration
- Consistent Error Handling
- Progressive Enhancement
- Host/Container Harmony
- Security Focus
