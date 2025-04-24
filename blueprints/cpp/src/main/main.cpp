// === src/main/main.cpp ===
// Main entry point for the C++ chatbot application.

#include "../chatbot_lib/chatbot.h" // Include the chatbot library header
#include <iostream>                // For console input/output (cin, cout)
#include <string>                  // For std::string
#include <limits>                  // For numeric_limits (used in input clearing)

int main() {
    // Create an instance of the Chatbot
    // The project name will be substituted here by devrs
    cmdev::chatbot::Chatbot bot("{PROJECT_NAME}");

    std::cout << "Chatbot '" << "{PROJECT_NAME}" << "' initialized." << std::endl;
    std::cout << "Type 'help' for commands, or 'bye' to exit." << std::endl;

    std::string user_input;

    while (true) {
        std::cout << "\nYou: ";

        // Read a whole line from the user
        if (!std::getline(std::cin, user_input)) {
            // Handle potential end-of-file or input errors
            if (std::cin.eof()) {
                std::cout << "\nInput stream closed. Exiting." << std::endl;
            } else {
                std::cerr << "\nError reading input." << std::endl;
                // Clear error flags and ignore the rest of the line
                std::cin.clear();
                std::cin.ignore(std::numeric_limits<std::streamsize>::max(), '\n');
            }
            break; // Exit loop on input error or EOF
        }

        // Check if the user wants to exit (case-insensitive check is handled in respond)
        if (user_input == "bye" || user_input == "Bye" || user_input == "BYE") { // Simple check here
            std::cout << "Bot: Goodbye!" << std::endl;
            break; // Exit the loop
        }

        // Get the bot's response
        std::string response = bot.respond(user_input);

        // Print the bot's response
        std::cout << "Bot: " << response << std::endl;
    }

    return 0; // Indicate successful execution
}

