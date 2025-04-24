# === Root CMakeLists.txt for C++ Blueprint ===
# Defines the overall project structure and settings.

# Minimum CMake version required
cmake_minimum_required(VERSION 3.15)

# Project definition
# The PROJECT_NAME variable will be replaced by 'devrs blueprint create'
project({PROJECT_NAME} VERSION 0.1.0 LANGUAGES CXX)

# Set the C++ standard (e.g., C++17 or C++20)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Add the source directory containing library and executable targets
add_subdirectory(src)

# --- Optional: Improve build output location ---
# Set default output directories for libraries and executables
set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)

# --- Optional: Enable warnings (recommended) ---
if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    add_compile_options(-Wall -Wextra -Wpedantic)
endif()

# --- Message to user ---
message(STATUS "Configuring ${PROJECT_NAME} version ${PROJECT_VERSION}")
message(STATUS "Build type: ${CMAKE_BUILD_TYPE}") # Set by CMakePresets or -D flag
message(STATUS "Output directory: ${CMAKE_BINARY_DIR}")


