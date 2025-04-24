# C++ Project Blueprint ({PROJECT_NAME})

This is a minimal C++ project blueprint using CMake.

## Structure

- `CMakeLists.txt`: Root CMake configuration.
- `CMakePresets.json`: Standard presets for configuration (Debug/Release).
- `Dockerfile`: Builds the application in a container.
- `src/`: Contains source code.
  - `chatbot_lib/`: A simple library with chatbot logic.
  - `main/`: The main executable using the library.

## Building

1.  **Configure using CMake Presets:**
    ```bash
    # From the project root directory
    cmake --preset debug # Or --preset release
    ```
2.  **Build:**
    ```bash
    cmake --build build --config Debug # Or --config Release
    ```
    *(The executable will be in `build/bin/`)*

## Running

```bash
./build/bin/{PROJECT_NAME}_app
```

## Docker

Build the image:
```bash
docker build -t {PROJECT_NAME_LOWER}:latest .
```

Run the container:
```bash
docker run -it --rm {PROJECT_NAME_LOWER}:latest
```

*(This README will be customized by `devrs blueprint create`)*

