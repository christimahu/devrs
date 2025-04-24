#![warn(missing_docs)]

//! Blueprint for new Rust projects
//! Author: Christi Mahu – <https://christimahu.dev>
//! Part of the dev repo: <https://github.com/christimahu/dev/>
//!
//! This crate defines a minimal idiomatic Rust blueprint for new CLI applications.
//! It introduces modular source layout, unit/integration tests, `Result`-based
//! error handling, and the use of common idioms like `unwrap()` and `expect()`.

// ===============================
// 📦 chatbot.rs - Core chatbot logic
// ===============================
//
// This module defines the `Chatbot` struct and its methods.
// It includes both a basic `respond()` method and a `try_respond()`
// method that introduces error handling via `Result<T, E>`.
//
// This is a great place to introduce core concepts like:
// - struct definition
// - method implementation via `impl`
// - string processing
// - idiomatic error handling with Result

/// A very basic chatbot implementation.
///
/// The `Chatbot` holds a name, and responds to a few fixed phrases.
/// It serves as a simple, focused example of how to structure logic in Rust.
pub struct Chatbot {
    name: String,
}

impl Chatbot {
    /// Creates and returns a new Chatbot with the given name.
    ///
    /// This is an idiomatic constructor in Rust: not called `new_bot()` or `make()`,
    /// just `new()`. You’ll see this pattern everywhere.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    /// Respond to user input with a fixed set of responses.
    ///
    /// This function never fails — it's simple and safe for casual use.
    /// Unknown input returns a fallback string.
    pub fn respond(&self, input: &str) -> String {
        match input.to_lowercase().trim() {
            "hi" | "hello" => "Hello! How can I help you today?".into(),
            "how are you?" => "I'm just code, but I'm running smoothly.".into(),
            "what's your name?" => format!("My name is {}.", self.name),
            "help" => "Try typing 'hi', 'how are you?', or 'what's your name?'."
                .into(),
            _ => "I'm not sure how to respond to that.".into(),
        }
    }

    /// Fallible version of `respond()` — returns a `Result`.
    ///
    /// - If input is valid, returns `Ok(String)` with a reply.
    /// - If input is empty or just whitespace, returns `Err(String)` with an error message.
    ///
    /// This introduces Rust’s core error-handling type: `Result<T, E>`.
    pub fn try_respond(&self, input: &str) -> Result<String, String> {
        let trimmed = input.trim();

        if trimmed.is_empty() {
            // ❌ Empty input isn't valid here — return an error
            return Err("Input cannot be empty.".into());
        }

        // ✅ Use the infallible version internally
        Ok(self.respond(trimmed))
    }
}

// ===============================
// 🧪 Inline Unit Tests
// ===============================
//
// Unit tests in Rust typically live in the same file as the code.
// These are compiled only when running `cargo test`.
// They're great for validating logic as your code grows.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responds_to_greetings() {
        // WHY: Greetings are a common user action. These tests verify consistent response
        // regardless of case or whitespace.
        let bot = Chatbot::new("TestBot");

        assert_eq!(bot.respond("hi"), "Hello! How can I help you today?");
        assert_eq!(bot.respond("HELLO"), "Hello! How can I help you today?");
        assert_eq!(bot.respond("  hi "), "Hello! How can I help you today?");
    }

    #[test]
    fn result_is_ok_for_valid_input() {
        // WHY: This tests the fallible method `try_respond()` and demonstrates how
        // to use `.expect()` safely. This is idiomatic when failure is unexpected.
        let bot = Chatbot::new("Rusty");

        let reply = bot
            .try_respond("how are you?")
            .expect("Chatbot failed to respond");

        assert_eq!(reply, "I'm just code, but I'm running smoothly.");
    }

    #[test]
    fn result_is_err_for_empty_input() {
        // WHY: Ensures that invalid (empty) input is rejected in `try_respond()`.
        // This is a key part of reliable user input handling.
        let bot = Chatbot::new("Rusty");

        let result = bot.try_respond("");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Input cannot be empty.");
    }

    #[test]
    fn respond_fallback_for_empty_input() {
        // WHY: Verifies that `respond()` falls back safely even for empty input.
        // Contrast this with the strictness of `try_respond()`.
        let bot = Chatbot::new("Rusty");

        assert_eq!(
            bot.respond(""),
            "I'm not sure how to respond to that."
        );
    }
}
