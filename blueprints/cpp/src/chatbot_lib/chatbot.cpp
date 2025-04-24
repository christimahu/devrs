// === src/chatbot_lib/chatbot.cpp ===
// Implementation file for the Chatbot class.

#include "chatbot.h" // Include the header file
#include <iostream>   // For potential logging (optional)
#include <algorithm>  // For std::transform (to_lower)
#include <cctype>     // For ::tolower

namespace cmdev {
namespace chatbot {

// Constructor implementation
Chatbot::Chatbot(const std::string& name) : bot_name_(name) {
    // Initialization logic, if any, goes here.
    // std::cout << "Chatbot '" << bot_name_ << "' initialized." << std::endl;
}

// Helper function to convert string to lower case and trim whitespace
std::string Chatbot::to_lower_and_trim(const std::string& str) const {
    std::string result = str;

    // Trim leading whitespace
    result.erase(result.begin(), std::find_if(result.begin(), result.end(), [](unsigned char ch) {
        return !std::isspace(ch);
    }));

    // Trim trailing whitespace
    result.erase(std::find_if(result.rbegin(), result.rend(), [](unsigned char ch) {
        return !std::isspace(ch);
    }).base(), result.end());

    // Convert to lower case
    std::transform(result.begin(), result.end(), result.begin(),
                   [](unsigned char c){ return std::tolower(c); });
    return result;
}


// respond method implementation
std::string Chatbot::respond(const std::string& input) {
    std::string processed_input = to_lower_and_trim(input);

    if (processed_input == "hi" || processed_input == "hello") {
        return "Hello there!";
    } else if (processed_input == "how are you?") {
        return "I'm a C++ program, I'm doing fine!";
    } else if (processed_input == "what's your name?" || processed_input == "what is your name?") {
        return "My name is " + bot_name_ + ".";
    } else if (processed_input == "help") {
         return "You can say 'hi', 'how are you?', or 'what's your name?'. Type 'bye' to exit.";
    }
    // Add more responses here if needed
    else {
        return "I didn't understand that. Try 'help'.";
    }
}

} // namespace chatbot
} // namespace cmdev

