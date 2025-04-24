# DevRS Architecture Overview

This document outlines the technical architecture and design principles of the DevRS CLI tool, including its core concepts, project structure, and implementation details.

## 1. Core Philosophy

DevRS aims to provide a consistent, powerful, and configurable development environment management system with these key goals:

* **Unified Interface:** Consolidate various development tasks (environment management, containerization, project scaffolding) under a single `devrs` command
* **Reproducibility:** Leverage Docker to ensure identical development environments across different machines
* **Flexibility:** Configure mounts, ports, environment variables, and template locations via TOML files
* **Extensibility:** Follow a modular design to facilitate future additions
* **Idiomatic Rust:** Utilize Rust best practices for error handling, async programming, and type safety

## 2. Core Concepts

DevRS organizes functionality around several key concepts:

### 2.1 Core Development Environment

The `devrs env` commands manage a single, consistent Docker container with development tools pre-installed:

* Provides a standardized environment with your complete toolchain
* Based on the `Dockerfile` in the DevRS repo root
* Operates as a long-running environment used across multiple projects
* Supports building, starting, executing commands, and viewing logs

Example usage:
```bash
# Start a shell in the development environment
devrs env shell

# Execute a command within the environment
devrs env exec cargo build

# Check environment status
devrs env status
```

### 2.2 Application Containers

The `devrs container` commands manage application-specific containers:

* Specific to individual applications you're developing
* Based on Dockerfiles in your actual projects
* Used for running your applications, not for development
* Support building images, running containers, managing lifecycle
* Typically short-lived or used for testing/running applications

Example usage:
```bash
# Build a container image from the current project
devrs container build --tag myapp:1.0

# Run a container from the built image
devrs container run --image myapp:1.0 --port 8080:80

# View container logs
devrs container logs myapp-instance
```

### 2.3 Project Templates (Blueprints)

The `devrs blueprint` commands manage project templates:

* Templates for creating starter code for various project types
* Support template variables (using Tera syntax) replaced during creation
* Include conditional logic and special file handling
* Detect project types (Rust, Go, etc.) and provide appropriate configuration

Example usage:
```bash
# List available templates
devrs blueprint list

# Create a new project from a template
devrs blueprint create --lang rust my-project

# View template details
devrs blueprint info rust
```

### 2.4 HTTP Server (srv)

The `devrs srv` command provides a simple HTTP file server:

* Serves static files from a specified directory
* Supports SPA (Single Page Application) mode by routing paths to index.html
* Configurable with CORS, port, and other options
* Automatically finds available ports if requested one is in use
* Shows network links for easy access from other devices

Example usage:
```bash
# Serve files from the current directory
devrs srv .

# Specify port and host interface
devrs srv --port 9000 --host 0.0.0.0 ./dist
```

### 2.5 Setup

The `devrs setup` commands configure the host system:

* Configures shell integration (`.bashrc`/`.zshrc`)
* Sets up Neovim configuration and plugins
* Checks for required dependencies (Docker, Git)
* Creates default configuration files

Example usage:
```bash
# Run all setup tasks
devrs setup

# Set up only shell integration
devrs setup shell
```

## 3. Project Structure

DevRS is organized with the following directory structure:

```
devrs/                         # Root workspace directory
├── .github/                   # GitHub Actions workflows
├── cli/                       # === The main CLI application crate ===
│   ├── src/
│   │   ├── main.rs            # Entry point, Clap setup, command routing
│   │   ├── commands/          # Command group implementations
│   │   │   ├── blueprint/     # Blueprint command implementations
│   │   │   ├── container/     # Container command implementations
│   │   │   ├── env/           # Environment command implementations
│   │   │   ├── setup/         # Setup command implementations
│   │   │   └── srv/           # HTTP server command implementation
│   │   ├── common/            # Shared utility functionality
│   │   │   ├── docker/        # Docker operations
│   │   │   ├── fs/            # Filesystem operations
│   │   │   ├── network/       # Network utilities
│   │   │   ├── process/       # Process execution
│   │   │   ├── system/        # System-level utilities
│   │   │   ├── ui/            # User interface helpers
│   │   │   └── archive/       # Archive handling (tar, etc.)
│   │   └── core/              # Core application infrastructure
│   │       ├── config.rs      # Configuration loading
│   │       ├── error.rs       # Error types
│   │       └── templating.rs  # Blueprint templating
├── config/                    # Configuration assets
│   ├── init.lua               # Neovim configuration
│   ├── shell_functions        # Shared shell functions
│   └── bashrc/zshrc           # Shell configurations
├── blueprints/                # Project template directories
│   ├── rust/                  # Rust project templates
│   ├── go/                    # Go project templates
│   └── cpp/                   # C++ project templates
├── Dockerfile                 # Core development environment definition
└── config_example.toml        # Example user configuration
```

## 4. Implementation Architecture

### 4.1 Command Processing Flow

1. **Entry Point (`main.rs`):**
   * Parses command-line arguments using Clap
   * Sets up logging based on verbosity
   * Routes to appropriate command handler

2. **Command Modules (`commands/`):**
   * Each module corresponds to a command group (e.g., `env`, `container`)
   * Modules define command-specific arguments using Clap
   * Handler functions implement command logic

3. **Command Execution:**
   * Command handlers typically:
     * Load and validate configuration
     * Interact with Docker via the `common/docker` module
     * Process files via the `common/fs` module
     * Return structured errors for consistent handling

### 4.2 Core Infrastructure

#### Configuration System (`core/config.rs`)

DevRS loads configuration from multiple sources, in order of precedence:

1. Command-line arguments (highest priority)
2. Project-specific `.devrs.toml` file in current directory
3. User configuration (`~/.config/devrs/config.toml`)
4. Default values (lowest priority)

Configuration settings include:
* Core environment container settings (image name, mounts, ports)
* Blueprint directory location
* Application container defaults

#### Error Handling (`core/error.rs`)

DevRS uses a structured approach to error handling:

* Custom error types with `thiserror` for detailed error information
* `anyhow` for error context and propagation
* Consistent error reporting to users

#### Templating System (`core/templating.rs`)

The templating system powers the blueprint functionality:

* Uses the Tera templating engine
* Handles directory traversal and file processing
* Applies variables to template files
* Copies non-template files directly

### 4.3 Common Utilities

#### Docker Operations (`common/docker`)

The Docker module abstracts interaction with the Docker Engine API:

* Image building and management
* Container creation and lifecycle
* Volume and port mapping
* Command execution within containers
* Logging and status reporting

Submodules include:
* `operations.rs` - Core Docker API operations
* `containers.rs` - Container-specific operations
* `images.rs` - Image-specific operations
* `utils.rs` - Docker-specific utilities

#### Filesystem Utilities (`common/fs`)

The filesystem module provides utilities for file and directory operations:

* Directory copying and creation
* Symbolic link management
* File reading and writing
* Path resolution and validation

#### Other Utilities

* `network` - Network-related utilities (port discovery, HTTP)
* `archive` - Creating and extracting archives (tar, etc.)
* `process` - Running external processes
* `system` - Detecting shells, tools, and system properties
* `ui` - User interface helpers for progress, tables, etc.

## 5. Development Workflow

DevRS is designed to support its own development process through:

1. **Clone and Setup:** Clone repository and run `devrs setup`
2. **Build Environment:** Run `devrs env build` to create core environment
3. **Development:** Use `devrs env shell` for development work
4. **Testing:** Run tests within environment for compatibility
5. **Application Building:** Use `devrs container build` for testing

## 6. Future Enhancements

Planned future enhancements include:

1. **Additional Templates:** More project templates for various languages/frameworks
2. **Enhanced UI:** Improved progress reporting and interactive prompts
3. **Remote Environments:** Support for remote development environments
4. **Plugin System:** Support for user-defined plugins to extend functionality
5. **Web Dashboard:** A web interface for managing development environments

## 7. Implementation Status

The current implementation includes:

* **Complete:**
  * Configuration loading and validation
  * Command-line argument parsing
  * Blueprint creation and listing
  * HTTP server functionality
  * Basic Docker operations
  * Error handling system

* **Partially Implemented:**
  * Container operations (build, run)
  * Blueprint detection and information
  * Docker API integration

* **Planned:**
  * Setup commands
  * Environment shell and execution
  * Environment status and management
  * Container management (logs, removal)
  * User interface improvements

## 8. Design Principles

DevRS follows these key design principles:

1. **Single Responsibility:** Each module has a clear, focused purpose
2. **Command Isolation:** Commands operate independently with clear interfaces
3. **Clean Configuration:** Configuration is validated and paths expanded
4. **Consistent Error Handling:** Structured errors with context
5. **Progressive Enhancement:** Core features first, advanced features later
6. **Host/Container Harmony:** Maintain consistent experience across environments
7. **Security Focus:** Principle of least privilege, avoid root in containers
