---
name: slash-commands
tags: [cli, repl, commands]
skills: [json-api]
expected: All slash commands work correctly with proper functionality
---

Test all available slash commands in the Arcee Code REPL interface.

The system should:
- `/help` - Display a comprehensive help message with all available commands
- `/clear` - Clear the conversation history and reset the REPL
- `/compact` - Compress conversation context to improve performance
- `/cost` - Show token usage and estimated cost for the current session
- `/model` - Show current model and allow switching between models
- `/intensity` - Set model routing intensity (high, medium, low)
- `/strictness` - Show or set permission strictness (high, medium, low)
- `/tokens` - Show estimated token count for the current conversation
- `/history` - Show conversation summary with key details
- `/quit` - Exit the arcee-code application gracefully

Each command should:
- Execute without errors
- Provide clear, user-friendly output
- Handle edge cases appropriately
- Return appropriate exit codes when needed