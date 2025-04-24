// Blueprint for new Rust projects
// Author: Christi Mahu – https://christimahu.dev
// Part of the dev repo: https://github.com/christimahu/dev/
// This file is part of a minimal idiomatic Rust blueprint for creating new applications.
//
// main.rs - The entry point for the chatbot application.
//
// This file sets up a REPL-style loop that allows the user to chat with the bot.
// It now uses `try_respond()` to demonstrate idiomatic `Result<T, E>` handling.

mod chatbot;

use std::io::{self, Write};

fn main() {
    let bot = chatbot::Chatbot::new("Rusty");

    println!("Chat with Rusty! Type 'bye' to quit.");

    loop {
        print!("You: ");

        match io::stdout().flush() {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Failed to flush stdout: {}", e);
                continue;
            }
        }

        let mut input = String::new();

        if io::stdin().read_line(&mut input).is_err() {
            println!("Rusty: I didn't understand that.");
            continue;
        }

        let trimmed = input.trim();
        if trimmed.eq_ignore_ascii_case("bye") {
            println!("Rusty: Goodbye!");
            break;
        }

        // 🧠 We now use the fallible `try_respond()` method instead of `respond()`.
        // This teaches how to handle a `Result<String, String>` using `match`.
        match bot.try_respond(trimmed) {
            Ok(reply) => println!("Rusty: {}", reply),
            Err(msg) => println!("Rusty (error): {}", msg),
        }
    }
}
