// === src/chatbot_lib/chatbot.h ===
// Header file for the simple Chatbot class.

#ifndef CMDEV_CHATBOT_H // Header guard to prevent multiple inclusions
#define CMDEV_CHATBOT_H

#include <string> // For std::string
#include <vector> // Potentially for more complex responses later

// Use a namespace to avoid naming conflicts
namespace cmdev {
namespace chatbot {

/**
 * @brief A simple chatbot class.
 *
 * Responds to a few predefined inputs with fixed replies.
 */
class Chatbot {
public:
    /**
     * @brief Constructor.
     * @param name The name the chatbot should use for itself.
     */
    explicit Chatbot(const std::string& name);

    /**
     * @brief Get a response based on user input.
     * @param input The user's input string.
     * @return The chatbot's reply string.
     */
    std::string respond(const std::string& input);

    // Destructor (default is fine here)
    ~Chatbot() = default;

    // Disable copy operations for simplicity, if needed
    Chatbot(const Chatbot&) = delete;
    Chatbot& operator=(const Chatbot&) = delete;
    // Enable move operations (optional, default is likely fine)
    Chatbot(Chatbot&&) = default;
    Chatbot& operator=(Chatbot&&) = default;


private:
    std::string bot_name_; // Stores the name provided during construction

    // Helper function for case-insensitive comparison or trimming (optional)
    std::string to_lower_and_trim(const std::string& str) const;
};

} // namespace chatbot
} // namespace cmdev

#endif // CMDEV_CHATBOT_H

