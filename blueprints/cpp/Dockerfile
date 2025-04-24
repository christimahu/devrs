# === Minimal Dockerfile for C++ Blueprint ===

# Use a base image with GCC, G++, CMake, and Make/Ninja
FROM ubuntu:22.04 AS builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    ninja-build \
    git \
 && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy source code into the image
COPY . .

# Configure using CMake (using Release preset by default)
# Assumes CMakePresets.json exists and defines a 'release' preset
RUN cmake --preset release -B build

# Build the project using the configured build system (Ninja)
RUN cmake --build build --config Release

# --- Final Stage ---
# Use a smaller base image if possible, or just use the build stage
# For simplicity here, we use the builder stage directly.
# If optimization is needed, copy only the compiled binary to a smaller runtime image.
FROM builder AS final

# Copy the built executable from the build stage
COPY --from=builder /app/build/bin/{PROJECT_NAME}_app /usr/local/bin/{PROJECT_NAME}_app

# Set the entrypoint or default command
WORKDIR /app
# Expose ports if needed (e.g., if it were a server)
# EXPOSE 8080

# Command to run the application
CMD ["{PROJECT_NAME}_app"]


