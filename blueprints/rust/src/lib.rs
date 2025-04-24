// Blueprint for new Rust projects
// Author: Christi Mahu – https://christimahu.dev
// Part of the dev repo: https://github.com/christimahu/dev/
// This file is part of a minimal idiomatic Rust blueprint for creating new applications.
//
// lib.rs - This file defines the public library interface for this project.
// It allows your modules (like chatbot.rs) to be used from integration tests
// and from other Rust projects if this ever becomes a crate.
//
// ✅ Required for tests in `tests/` to `use chatbot::...`
// ✅ Common in real-world projects that need both a binary (main.rs) and library (lib.rs)

pub mod chatbot;
