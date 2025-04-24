# 🦀 DevRS ⚙️: Containerized Development Environment & Tooling

**DevRS** provides a unified, cross-platform development environment focused on consistent, reproducible workflows powered by Docker.

[![License: MPL 2.0](https://img.shields.io/badge/License-MPL_2.0-brightgreen.svg)](LICENSE)

## Overview

Working across multiple projects often requires juggling different language versions, dependencies, and toolchains. DevRS solves this by leveraging Docker to create isolated, reproducible development environments controlled through a single CLI tool.

## Key Features

* **Unified Command-Line Interface:** Manage your core development environment, project-specific containers, and templates with a single `devrs` command.
* **Containerized Core Environment:** Keep your host system clean while accessing a rich, pre-configured development environment with `devrs env shell`. Includes tools for Rust, C/C++, Go, Python, Node.js, cloud SDKs, and more.
* **Project Templates:** Quickly scaffold new projects using built-in or custom templates via `devrs blueprint create`.
* **Application Container Management:** Build, run, and manage Docker containers for your applications with `devrs container`, separate from your core environment.
* **Static File Server:** Includes `devrs srv`, a simple HTTP file server with CORS support and SPA routing for local development.
* **Consistent Configuration:** Share environment settings between host and container via TOML-based configuration.

## Installation
> ⚠️ **DISCLAIMER:**  
> **This repository is in the early phases of being rewritten and is not suitable for production development yet.**

1. **Clone the Repository:**
   ```bash
   # Recommended location: a dedicated tools directory
   git clone <repository_url> ~/tools/devrs
   cd ~/tools/devrs
   ```

2. **Build and Install DevRS:**
   ```bash
   # Install the DevRS binary from the cli crate
   cargo install --path ./cli
   ```
   *This places the `devrs` binary in `~/.cargo/bin/`. Ensure this is in your PATH.*

3. **Verify Installation:**
   ```bash
   devrs --version
   ```

## Initial Setup

After installing the `devrs` binary, configure your host system:

```bash
devrs setup
```

This command:
- Creates config directory at `~/.config/devrs/`
- Adds shell integration to your `.bashrc` or `.zshrc`
- Sets up Neovim configuration and plugins
- Checks for dependencies like Docker and Git

**Important:** After setup completes, reload your shell configuration for changes to take effect:

```bash
# For Bash users
source ~/.bashrc

# For Zsh users
source ~/.zshrc
```

## Build the Core Environment

Before first use, build the core development environment Docker image:

```bash
devrs env build
```

This creates a Docker image with a comprehensive set of development tools based on the Dockerfile in the repo.

## Core Commands

### Development Environment (`devrs env`)

Manage your primary development environment container:

```bash
# Start a shell in the development environment
devrs env shell

# Run a command in the environment without starting a shell
devrs env exec cargo build

# Check environment status
devrs env status
```

### Project Templates (`devrs blueprint`)

Work with project templates for rapid project scaffolding:

```bash
# List available templates
devrs blueprint list

# Create a new Rust project
devrs blueprint create --lang rust my-project

# Get detailed info about a template
devrs blueprint info rust
```

### Application Containers (`devrs container`)

Manage application-specific containers:

```bash
# Build a container image from the current directory
devrs container build --tag myapp:1.0

# Run a container from the image
devrs container run --image myapp:1.0 --port 8080:80

# View container logs
devrs container logs my-container
```

### Static File Server (`devrs srv`)

Serve files from any directory:

```bash
# Serve the current directory on port 8000
devrs srv .

# Specify a port and host interface
devrs srv --port 9000 --host 0.0.0.0 ./dist
```

## Configuration

DevRS uses TOML configuration files in standard locations:

**User Configuration:** `~/.config/devrs/config.toml`  
**Project Configuration:** `.devrs.toml` in your project directory (overrides user config)

**Example Configuration:**

```toml
[core_env]
# Mount directories (HOST_PATH -> CONTAINER_PATH)
mounts = [
  { host = "~/code", container = "/home/me/code", readonly = false },
  { host = "~/.ssh", container = "/home/me/.ssh", readonly = true },
]
# Port mappings (HOST:CONTAINER)
ports = ["8080:8080", "5173:5173"]
# Container environment variables
[core_env.env_vars]
RUST_LOG = "info"
# Core image settings
image_name = "devrs-core-env"
image_tag = "latest"

[blueprints]
# Path to blueprint templates
directory = "~/tools/devrs/blueprints"
```

See the full commented [config_example.toml](config_example.toml) for all available options.

## Project Documentation

For more details about DevRS, see:

- [ARCHITECTURE.md](ARCHITECTURE.md) - Technical architecture and implementation details
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Common issues and solutions
- [CONTRIBUTING.md](CONTRIBUTING.md) - Guidelines for contributing
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) - Community guidelines

## License

DevRS is licensed under the Mozilla Public License 2.0 (MPL-2.0). See the [LICENSE](LICENSE) file for details.
