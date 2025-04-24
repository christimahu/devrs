// Blueprint for new Go projects
// Author: Christi Mahu – https://christimahu.dev
// Part of the dev repo: https://github.com/christimahu/dev/
// This file is part of a minimal idiomatic Go blueprint for creating new applications.
//
// chatbot.go - A trivial chatbot that responds to common phrases.
// This file demonstrates how to define structs and methods in Go.

package chatbot

import "strings"

// Bot is a chatbot that knows its name and replies to a few known inputs.
type Bot struct {
	Name string
}

// NewBot returns a new Bot instance with the provided name.
// This is the idiomatic Go approach for constructors.
func NewBot(name string) *Bot {
	return &Bot{Name: name}
}

// Respond returns a simple reply string for known inputs.
// Any unknown input returns a default response.
func (b *Bot) Respond(input string) string {
	switch strings.ToLower(strings.TrimSpace(input)) {
	case "hi", "hello":
		return "Hello! How can I help you today?"
	case "how are you?":
		return "I'm just code, but I'm functioning as expected!"
	case "what's your name?":
		return "My name is " + b.Name + "."
	case "help":
		return "You can say things like 'hi', 'how are you', or 'what's your name'."
	default:
		return "I'm not sure how to respond to that."
	}
}
