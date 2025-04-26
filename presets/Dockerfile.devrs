# ==============================================================================
# Dockerfile for the DevRS Core Development Environment
# ==============================================================================
# This Dockerfile uses a multi-stage build to create a comprehensive development
# environment based on Ubuntu 22.04. It installs various languages, tools,
# cloud SDKs, and sets up a non-root user for development work.
#
# Stages:
# 1. system-setup: Installs all system-level packages and dependencies.
# 2. user-setup: Creates a non-root user, installs user-specific tools (Rust, Go tools), and sets up Neovim/Packer.
# 3. dev-environment: Final stage, configures the environment (PATH, TERM) and sets the default user/command.
# ==============================================================================

# Change this value to "amd64" when building on Linux or WSL
ARG ARCH="arm64"

# ==============================================================================
# Stage 1: System-level Setup (as root)
# ==============================================================================
# Start with a stable Ubuntu base image. 22.04 LTS provides good support.
FROM ubuntu:22.04 AS system-setup

# Set DEBIAN_FRONTEND to noninteractive to prevent apt commands from prompting for user input during the build.
ENV DEBIAN_FRONTEND=noninteractive

# --- Install Core Development Tools & Utilities ---
# Update package lists and install essential development tools, compilers,
# utilities, and language runtimes available via apt.
# --fix-missing can help with transient network issues during apt update.
# Using a single RUN command with '&& \' reduces layer count.
RUN apt-get update --fix-missing && \
    apt-get install -y \
    # Core Utils: Common command-line utilities
    curl wget git vim neovim tmux tree \
    # Build Tools: Essential for compiling C/C++/etc. projects
    build-essential gcc g++ make cmake ninja-build entr \
    # C++ Tools: Clang toolchain and related utilities
    clang clang-format clang-tidy clangd \
    llvm lldb lld libc++-dev libc++abi-dev \
    # Python: Python 3 runtime, pip, venv support, and common data science libs
    python3 python3-pip python3-venv \
    python3-numpy python3-pandas python3-matplotlib python3-scipy python3-sklearn \
    # Networking/Security: Needed for adding external repositories securely
    apt-transport-https ca-certificates gnupg \
    # Search Tools: Fast text search tools
    ripgrep fd-find \
    # Shells: Zsh shell as an alternative to bash
    zsh \
    # Build Dependencies: Often required by various build systems
    pkg-config libssl-dev \
    # Documentation Tools: For generating documentation
    graphviz doxygen \
    # LuaJIT: Often required for Neovim plugins performance
    luajit libluajit-5.1-dev lua5.1 liblua5.1-0-dev \
    # Clean up apt cache to reduce image size
    && rm -rf /var/lib/apt/lists/*

# --- Install Node.js ---
# Use the official NodeSource repository for a recent version of Node.js (v20.x).
# This involves adding the NodeSource GPG key and repository definition.
RUN apt-get update && \
    apt-get install -y ca-certificates curl gnupg && \
    mkdir -p /etc/apt/keyrings && \
    curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key | gpg --dearmor -o /etc/apt/keyrings/nodesource.gpg && \
    echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_20.x nodistro main" | tee /etc/apt/sources.list.d/nodesource.list && \
    apt-get update && \
    # Install Node.js itself
    apt-get install -y nodejs && \
    # Install pnpm globally via npm
    npm install -g pnpm && \
    # Clean up apt cache
    rm -rf /var/lib/apt/lists/*

# --- Install Go ---
# Download the official Go binary tarball directly from go.dev for the specified architecture
ARG ARCH
RUN set -eux; \
    GO_VERSION=1.21.5; \
    echo "Installing Go ${GO_VERSION} for ${ARCH}"; \
    URL="https://go.dev/dl/go${GO_VERSION}.linux-${ARCH}.tar.gz"; \
    curl -fsSLo go.tgz "${URL}" || (echo "Failed to download Go" && exit 1); \
    tar -C /usr/local -xzf go.tgz; \
    rm go.tgz; \
    /usr/local/go/bin/go version

# --- Install Google Cloud SDK ---
# Add the Google Cloud SDK package repository and install the core SDK
# along with several useful components (App Engine environments, emulators, kubectl).
RUN echo "deb [signed-by=/usr/share/keyrings/cloud.google.gpg] http://packages.cloud.google.com/apt cloud-sdk main" | \
    tee -a /etc/apt/sources.list.d/google-cloud-sdk.list && \
    curl https://packages.cloud.google.com/apt/doc/apt-key.gpg | \
    apt-key --keyring /usr/share/keyrings/cloud.google.gpg add - && \
    apt-get update && apt-get install -y google-cloud-sdk \
    google-cloud-sdk-app-engine-python \
    google-cloud-sdk-app-engine-python-extras \
    google-cloud-sdk-app-engine-java \
    google-cloud-sdk-app-engine-go \
    google-cloud-sdk-datastore-emulator \
    google-cloud-sdk-pubsub-emulator \
    google-cloud-sdk-bigtable-emulator \
    google-cloud-sdk-firestore-emulator \
    kubectl && \
    # Clean up apt cache
    rm -rf /var/lib/apt/lists/*

# --- Install Azure CLI ---
# Use Microsoft's official convenience script to install the Azure CLI.
RUN curl -sL https://aka.ms/InstallAzureCLIDeb | bash

# --- Install Global Node.js Packages ---
# Install commonly used Node.js tools globally using npm.
RUN npm install -g typescript ts-node eslint prettier

# --- Install Global Python Packages ---
# Install commonly used Python tools and libraries globally using pip3.
# --no-cache-dir reduces image size by not storing the pip cache.
RUN pip3 install --no-cache-dir \
    black isort mypy pytest jupyterlab ipython \
    pandas numpy matplotlib seaborn scikit-learn \
    google-api-python-client google-auth google-cloud-storage

# ==============================================================================
# Stage 2: User Setup (as non-root user 'me')
# ==============================================================================
# Start from the previous stage where all system tools are installed.
FROM system-setup AS user-setup

# --- Create Non-Root User ---
# Create a standard user 'me' with a home directory and bash shell.
# Also create essential directories needed for configuration and tools.
# Running as non-root is crucial for security and realistic development.
RUN useradd -m -s /bin/bash me && \
    mkdir -p /home/me/.dev/config && \
    mkdir -p /home/me/.config/nvim && \
    # Ensure the new user owns their home directory and its contents.
    chown -R me:me /home/me

# --- Neovim Config & Packer Setup ---
# Copy the user's Neovim configuration (init.lua) into the image *before*
# attempting to install plugins. This ensures Packer knows what to install.
# Assumes 'init.lua' is in a 'config' directory relative to the Dockerfile's build context.
COPY --chown=me:me config/init.lua /home/me/.config/nvim/init.lua

# --- Switch to Non-Root User ---
# Subsequent commands will run as the user 'me'.
USER me
WORKDIR /home/me

# --- Install Rust ---
# Use rustup (the official Rust toolchain installer) for installation.
# The '-y' flag skips interactive prompts.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# --- Install Rust Components & Common Crates ---
# Source the cargo environment script to make cargo/rustup available in this RUN command.
# Add essential components like rustfmt (formatter), clippy (linter), and rust-analyzer (LSP).
# Install some useful cargo helper tools globally within the user's context.
RUN . /home/me/.cargo/env && \
    rustup component add rustfmt clippy rust-analyzer && \
    cargo install cargo-watch cargo-edit cargo-expand tokio-console

# --- Install Go Development Tools ---
# Install Go language server (gopls) and other common Go development tools.
# Temporarily disable CGO to avoid needing a C compiler for these tool installations,
# simplifying the build process.
ENV CGO_ENABLED=0
RUN /usr/local/go/bin/go install golang.org/x/tools/gopls@v0.14.2 && \
    /usr/local/go/bin/go install github.com/go-delve/delve/cmd/dlv@v1.21.0 && \
    /usr/local/go/bin/go install github.com/fatih/gomodifytags@v1.16.0 && \
    /usr/local/go/bin/go install github.com/golangci/golangci-lint/cmd/golangci-lint@v1.55.2

# --- Setup Neovim Packer & Install Plugins ---
# Create the directory structure Packer expects.
# Clone the Packer plugin manager itself.
# Run Neovim headlessly to trigger PackerSync, which installs plugins defined in init.lua.
# The 'autocmd User PackerComplete quitall' makes Neovim exit automatically after Packer finishes.
# Use '|| true' to prevent the Docker build from failing if PackerSync encounters non-critical errors
# (like warnings about optional providers missing).
RUN mkdir -p /home/me/.local/share/nvim/site/pack/packer/start && \
    git clone --depth 1 https://github.com/wbthomason/packer.nvim /home/me/.local/share/nvim/site/pack/packer/start/packer.nvim

# TODO: does not complete when adding to the previous command 
# && \ nvim --headless -c 'autocmd User PackerComplete quitall' -c 'PackerSync' || true

# ==============================================================================
# Stage 3: Final Environment Configuration
# ==============================================================================
# Start from the user-setup stage, which contains all tools and user configurations.
FROM user-setup AS dev-environment

# --- Expose Common Development Ports ---
# Declare ports commonly used during development. This doesn't publish them,
# but serves as documentation and allows easier mapping when running the container.
EXPOSE 8000 8097 8098 8099 5173

# --- Set Environment Variables ---
# Add Go, Cargo, and local user binaries to the PATH.
# Set a suitable TERM variable for terminal compatibility (e.g., with tmux/nvim).
# Reset CGO_ENABLED to 1 for normal Go compilation that might require C linkage.
ENV PATH="/usr/local/go/bin:/home/me/.cargo/bin:/home/me/.local/bin:${PATH}"
ENV TERM=xterm-256color
ENV CGO_ENABLED=1

# --- Final Adjustments (as root) ---
# Switch back to root temporarily for system-level changes.
USER root
# Create a symbolic link so 'vim' command executes 'nvim'.
RUN ln -sf /usr/bin/nvim /usr/local/bin/vim

# --- Switch Back to User 'me' ---
# Ensure the container runs as the non-root user by default.
USER me
# Optionally add the vim alias to .bashrc for convenience, although shell_functions might handle this.
# RUN echo 'alias vim=nvim' >> /home/me/.bashrc

# --- Set Default Command ---
# When the container starts without a specific command, drop into a bash shell.

# Since not calling shell on build, tail is used instead to keep the container alive. 
# CMD ["/bin/bash"]

# Option 1: Use tail (very common)
CMD ["tail", "-f", "/dev/null"]
