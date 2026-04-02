---
name: thinking-mode
tags: [ui, thinking, ai, interaction]
skills: [json-api]
expected: Activates thinking mode with proper indicators and generates responses
---

Test the thinking mode functionality.

The system should:
- Display a thinking indicator (e.g., "Thinking..." or animated spinner)
- Process the prompt and generate a response
- Return a code output or explanation
- The thinking mode should be visually distinct from normal operation
- Support different thinking modes (light, medium, deep)
- Handle long-running thinking operations with progress indicators
- Allow cancellation of thinking operations
- Cache thinking results for similar prompts
- Support streaming thinking output
- Handle thinking interruptions gracefully