---
name: slash-simplify
tags: [cli, code, simplification, repl]
skills: [json-api]
expected: Simplifies code by refactoring and optimizing via slash command
---

Test the `/simplify` slash command for code simplification.

The system should:
- Accept file path or code snippet to simplify
- Read the target file(s) or code
- Identify:
  - Over-abstracted code that could be inlined
  - Unnecessary wrapper types or helper functions
  - Redundant error handling or validation
  - Dead code or unused imports
  - Complex control flow that could be simplified
  - Premature optimizations
- For each finding:
  - Explain why it's over-complex
  - Show the simplified version
  - Confirm the simplification preserves behavior
- Apply changes if the user approves
- Handle invalid file paths gracefully
- Handle syntax errors appropriately
- Provide clear explanations and diff output
- Support interactive confirmation for changes