// Blueprint for new Rust projects
// Author: Christi Mahu – https://christimahu.dev
// Part of the dev repo: https://github.com/christimahu/dev/
// This file is part of a minimal idiomatic Rust blueprint for creating new applications.
//
// tests/chatbot_tests.rs - Integration tests for the chatbot module.
//
// ✅ FIXED: This now correctly imports Chatbot using the full module path
// `chatbot::chatbot::Chatbot`
//
// These tests treat the chatbot like a “black box” — using only its public API.
// That means we must import the struct using the full path, starting with the crate root.

use chatbot::chatbot::Chatbot;

#[test]
fn test_greetings() {
    // WHY: Users often start with simple greetings like “hi” or “hello”.
    // This test ensures consistent bot behavior regardless of case or spacing.
    let bot = Chatbot::new("TestBot");

    assert_eq!(bot.respond("hi"), "Hello! How can I help you today?");
    assert_eq!(bot.respond("HELLO"), "Hello! How can I help you today?");
    assert_eq!(bot.respond("  hi "), "Hello! How can I help you today?");
}

#[test]
fn test_status_response() {
    // WHY: This tests a stable system message that doesn’t change over time.
    // Useful for ensuring refactors don’t alter expected bot replies.
    let bot = Chatbot::new("TestBot");

    assert_eq!(
        bot.respond("how are you?"),
        "I'm just code, but I'm running smoothly."
    );
}

#[test]
fn test_dynamic_name_usage() {
    // WHY: This validates string formatting and field access inside the struct.
    // You want to ensure the bot responds with the correct name.
    let bot = Chatbot::new("Rusty");

    assert_eq!(bot.respond("what's your name?"), "My name is Rusty.");
}

#[test]
fn test_help_command() {
    // WHY: Help text is essential UX. If it breaks, users won't know how to proceed.
    let bot = Chatbot::new("TestBot");

    assert_eq!(
        bot.respond("help"),
        "Try typing 'hi', 'how are you?', or 'what's your name?'."
    );
}

#[test]
fn test_unknown_inputs() {
    // WHY: This ensures graceful fallback behavior for unrecognized or weird input.
    // It avoids crashes, silent failure, or confusing errors.
    let bot = Chatbot::new("TestBot");

    assert_eq!(bot.respond("tell me a joke"), "I'm not sure how to respond to that.");
    assert_eq!(bot.respond("What is the meaning of life?"), "I'm not sure how to respond to that.");
    assert_eq!(bot.respond(""), "I'm not sure how to respond to that.");
}

#[test]
fn test_try_respond_success() {
    // WHY: Confirms the fallible method `try_respond()` works with good input.
    let bot = Chatbot::new("Rusty");

    let result = bot.try_respond("what's your name?");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "My name is Rusty.");
}

#[test]
fn test_try_respond_error() {
    // WHY: Empty input should be considered an error when using `try_respond()`.
    // This pattern helps teach `Result<T, E>` and safe error handling.
    let bot = Chatbot::new("Rusty");

    let result = bot.try_respond("    ");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Input cannot be empty.");
}
