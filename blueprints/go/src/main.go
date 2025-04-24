// Blueprint for new Go projects
// Author: Christi Mahu – https://christimahu.dev
// Part of the dev repo: https://github.com/christimahu/dev/
// This file is part of a minimal idiomatic Go blueprint for creating new applications.
//
// main.go - Entry point for the chatbot application. Accepts user input and
// responds with canned responses via the chatbot package.

package main

import (
	"bufio"
	"fmt"
	"os"
	"strings"

	"github.com/christimahu/dev/blueprints/go/src/chatbot"
)

func main() {
	bot := chatbot.NewBot("GoBot")

	fmt.Println("Chat with GoBot! Type 'bye' to exit.")
	scanner := bufio.NewScanner(os.Stdin)

	for {
		fmt.Print("You: ")
		if !scanner.Scan() {
			break
		}
		input := strings.TrimSpace(scanner.Text())
		if strings.ToLower(input) == "bye" {
			fmt.Println("GoBot: Goodbye!")
			break
		}
		fmt.Println("GoBot:", bot.Respond(input))
	}
}
