// Blueprint for new Go projects
// Author: Christi Mahu – https://christimahu.dev
// Part of the dev repo: https://github.com/christimahu/dev/
// This file is part of a minimal idiomatic Go blueprint for creating new applications.
//
// chatbot_test.go - Unit tests for the chatbot package using Testify for
// expressive assertions. These tests demonstrate edge case handling and
// expected behavior for all known inputs.

package chatbot

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

// Tests that known greeting inputs return the correct greeting response.
// These are common entry points into user interaction.
func TestRespond_Greetings(t *testing.T) {
	bot := NewBot("TestBot")
	assert.Equal(t, "Hello! How can I help you today?", bot.Respond("hi"))
	assert.Equal(t, "Hello! How can I help you today?", bot.Respond("hello"))
	assert.Equal(t, "Hello! How can I help you today?", bot.Respond("  Hi  ")) // whitespace trimmed
}

// Tests the bot's self-description when asked how it's doing.
// Meant to humanize the bot with a consistent, non-emotional response.
func TestRespond_Status(t *testing.T) {
	bot := NewBot("TestBot")
	assert.Equal(t, "I'm just code, but I'm functioning as expected!", bot.Respond("how are you?"))
}

// Verifies the bot replies with its configured name correctly.
func TestRespond_Name(t *testing.T) {
	bot := NewBot("TestBot")
	assert.Equal(t, "My name is TestBot.", bot.Respond("what's your name?"))
}

// Ensures the help message informs the user of valid inputs.
func TestRespond_Help(t *testing.T) {
	bot := NewBot("TestBot")
	expected := "You can say things like 'hi', 'how are you', or 'what's your name'."
	assert.Equal(t, expected, bot.Respond("help"))
}

// Tests a variety of unrecognized inputs.
// These validate fallback behavior for unknown or malformed queries.
func TestRespond_Unknown(t *testing.T) {
	bot := NewBot("TestBot")

	assert.Equal(t, "I'm not sure how to respond to that.", bot.Respond("tell me a joke"))
	assert.Equal(t, "I'm not sure how to respond to that.", bot.Respond("  ")) // empty input
	assert.Equal(t, "I'm not sure how to respond to that.", bot.Respond("42?"))
	assert.Equal(t, "I'm not sure how to respond to that.", bot.Respond("What's your favorite color?"))
}
