# ==========================================================================
# Go Project Makefile
# Author: Christi Mahu – https://christimahu.dev
# ==========================================================================
#
# This Makefile provides a complete workflow for building, testing, packaging,
# and deploying a Go application in Docker. Each command handles dependencies
# and provides clear feedback, asking for confirmation before destructive actions.
#
# WORKFLOW:
# 1. `make build` - Compiles the Go project
# 2. `make test` - Runs all tests
# 3. `make package` - Prepares artifacts for containerization
# 4. `make deploy` - Builds a Docker container
# 5. `make run` - Runs the application in Docker
# 6. `make clean` - Removes all build artifacts and containers
# 7. `make docs` - Generates documentation
#
# Each command checks if its dependencies have been completed and
# automatically runs them if needed (after confirmation).

# Application name
APP := chatbot

# Docker image name
IMAGE := $(APP)

# Docker container name
CONTAINER := $(APP)-container

# ==========================================================================
# Main Commands (in logical order)
# ==========================================================================

# When just running 'make' with no explicit command.
.DEFAULT_GOAL := build

# Shows help information about available commands
help:
	@echo "Go Project Makefile Help"
	@echo "======================="
	@echo
	@echo "Available commands:"
	@echo
	@echo "  make clean   - Removes all build artifacts and containers"
	@echo "  make test    - Runs all tests"
	@echo "  make build   - Compiles the project"
	@echo "  make package - Packages the binary for containerization"
	@echo "  make deploy  - Builds the Docker container"
	@echo "  make run     - Runs the application in Docker"
	@echo "  make docs    - Generates and serves documentation"
	@echo "  make help    - Shows this help message"
	@echo
	@echo "Commands can be run in any order - they'll automatically"
	@echo "run the necessary steps in sequence."

# Clean up all build artifacts, package, and Docker container
clean:
	@echo "This will remove all build artifacts, packages, and containers."
	@read -p "Are you sure? [y/N] " answer; \
	if [ "$$answer" = "y" ] || [ "$$answer" = "Y" ]; then \
		echo "Cleaning project..."; \
		rm -f $(APP); \
		rm -rf package; \
		docker rm -f $(CONTAINER) 2>/dev/null || true; \
		docker rmi -f $(IMAGE) 2>/dev/null || true; \
		echo "Project cleaned successfully."; \
	else \
		echo "Clean operation cancelled."; \
	fi

# Run all tests
test:
	@echo "Running tests..."
	go test ./...

# Build the application
build:
	@echo "Building $(APP)..."
	go build -o $(APP) ./src

# Package the application for Docker deployment
package: check_build
	@echo "Packaging $(APP) for deployment..."
	@mkdir -p package
	@cp $(APP) package/
	@cp Dockerfile package/
	@sed -i 's|/app/app|/app/$(APP)|g' package/Dockerfile
	@echo "Packaging complete. Files prepared in ./package/"

# Build the Docker container
deploy: check_package
	@echo "Building Docker container..."
	docker build -t $(IMAGE) ./package
	@echo "Docker container $(IMAGE) built successfully."

# Run the application in Docker
run: check_deploy
	@echo "Running $(APP) in Docker..."
	docker run --rm -it --name $(CONTAINER) $(IMAGE)

# Generate and serve API documentation
docs:
	@echo "Starting documentation server at http://localhost:6060"
	@echo "Press Ctrl+C to exit"
	godoc -http=:6060

# ==========================================================================
# Helper Commands (for dependency checking)
# ==========================================================================

# Check if build is complete
check_build:
	@if [ ! -f "$(APP)" ]; then \
		echo "Binary not found. Build needed first."; \
		read -p "Would you like to build now? [Y/n] " answer; \
		if [ -z "$$answer" ] || [ "$$answer" = "y" ] || [ "$$answer" = "Y" ]; then \
			$(MAKE) build; \
		else \
			echo "Operation cancelled."; \
			exit 1; \
		fi; \
	fi

# Check if package is complete
check_package:
	@if [ ! -d "package" ] || [ ! -f "package/$(APP)" ]; then \
		echo "Package not found. Packaging needed first."; \
		read -p "Would you like to package now? [Y/n] " answer; \
		if [ -z "$$answer" ] || [ "$$answer" = "y" ] || [ "$$answer" = "Y" ]; then \
			$(MAKE) package; \
		else \
			echo "Operation cancelled."; \
			exit 1; \
		fi; \
	fi

# Check if Docker image is built
check_deploy:
	@if [ -z "$(shell docker images -q $(IMAGE) 2>/dev/null)" ]; then \
		echo "Docker image not found. Deployment needed first."; \
		read -p "Would you like to deploy now? [Y/n] " answer; \
		if [ -z "$$answer" ] || [ "$$answer" = "y" ] || [ "$$answer" = "Y" ]; then \
			$(MAKE) deploy; \
		else \
			echo "Operation cancelled."; \
			exit 1; \
		fi; \
	fi

# Declare phony targets (not files)
.PHONY: help clean test build package deploy run docs check_build check_package check_deploy
