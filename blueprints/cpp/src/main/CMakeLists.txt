# === src/main/CMakeLists.txt ===
# Defines the main executable target.

# Define the executable name using the project prefix
# {PROJECT_NAME} will be replaced by 'devrs blueprint create'
set(APP_NAME "{PROJECT_NAME}_app")

# Create the executable target from main.cpp
add_executable(${APP_NAME}
    main.cpp
)

# Set the required C++ standard for this target (matches the library)
target_compile_features(${APP_NAME} PRIVATE cxx_std_17) # Or cxx_std_20 if you used that

# Link the executable against the chatbot library
# The library target name "{PROJECT_NAME}_lib" was defined in src/chatbot_lib/CMakeLists.txt
target_link_libraries(${APP_NAME}
    PRIVATE
        "{PROJECT_NAME}_lib" # Link to our static chatbot library
)

message(STATUS "Configuring executable: ${APP_NAME}")

# Ensure the executable is placed in the 'bin' directory within the build folder
set_target_properties(${APP_NAME} PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin"
)

