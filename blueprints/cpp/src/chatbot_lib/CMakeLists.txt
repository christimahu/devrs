# === src/chatbot_lib/CMakeLists.txt ===
# Defines the static library target for the chatbot logic.

# Define the library name using the project prefix
set(LIB_NAME "{PROJECT_NAME}_lib")

# Create the static library target
add_library(${LIB_NAME} STATIC
    chatbot.cpp
    chatbot.h
)

# Set the required C++ standard for this target
target_compile_features(${LIB_NAME} PUBLIC cxx_std_17) # Or cxx_std_20

# Specify that the current source directory should be included
# when other targets link against this library. This makes
# #include "chatbot.h" work correctly.
target_include_directories(${LIB_NAME}
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}>
        $<INSTALL_INTERFACE:include> # For potential installation
)

# Optional: Set properties like output name if needed
# set_target_properties(${LIB_NAME} PROPERTIES OUTPUT_NAME "chatbot")

message(STATUS "Configuring library: ${LIB_NAME}")


