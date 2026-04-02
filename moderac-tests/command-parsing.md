---
name: command-parsing
tags: [cli, parsing, arguments]
skills: [json-api]
author: Arcee Coder Team
created: 2025-01-01
status: active
priority: high
---

## Description
Test various CLI commands and their parsing behavior, including both command-line flags and REPL slash commands, ensuring robust argument handling and error recovery.

## Test Scenarios

### Scenario 1: Basic CLI Command Parsing
**Input:**
```json
{
  "prompt": "arcee \"write code for X\"",
  "context": {},
  "options": {}
}
```

**Expected Output:**
```json
{
  "response": "```rust\n// Generated code for X\n```",
  "metadata": {
    "session_id": "test-session-1",
    "prompt_analyzed": true,
    "code_generated": true
  }
}
```

**Validation Criteria:**
- [ ] Prompt correctly extracted from quotes
- [ ] Session created successfully
- [ ] AI response generated
- [ ] Code syntax valid
- [ ] Error handling for invalid prompts

**Edge Cases To Consider:**
- [ ] Empty prompt
- [ ] Missing quotes with spaces
- [ ] Special characters in prompt
- [ ] Very long prompts (>1000 chars)
- [ ] Invalid flags combined with prompt

### Scenario 2: REPL Slash Command Execution
**Input:**
```json
{
  "prompt": "/help",
  "context": {},
  "options": {
    "repl_mode": true
  }
}
```

**Expected Output:**
```json
{
  "response": "Available commands:\n/help - Show this help message\n/clear - Clear conversation history\n/compact - Compress context\n/cost - Show token usage\n/model - Show/change model\n/intensity - Set routing intensity\n/strictness - Show/set permission strictness\n/tokens - Show token count\n/history - Show conversation summary\n/quit - Exit application",
  "metadata": {
    "repl_command": true,
    "command_executed": "help",
    "exit_code": 0
  }
}
```

**Validation Criteria:**
- [ ] Command recognized and executed
- [ ] Output contains all available commands
- [ ] Proper formatting
- [ ] Exit code 0
- [ ] REPL mode maintained

**Edge Cases To Consider:**
- [ ] Unknown slash command
- [ ] Command with arguments
- [ ] Command history navigation
- [ ] Command execution errors

### Scenario 3: Combined Flags and REPL Commands
**Input:**
```json
{
  "prompt": "--mode thinking \"explain Y\"",
  "context": {},
  "options": {
    "repl_mode": true
  }
}
```

**Expected Output:**
```json
{
  "response": "Thinking mode activated...\nExplanation of Y: ...",
  "metadata": {
    "repl_command": false,
    "thinking_mode": true,
    "response_generated": true
  }
}
```

**Validation Criteria:**
- [ ] Flags parsed correctly
- [ ] Thinking mode activated
- [ ] Prompt processed
- [ ] Response generated
- [ ] Proper error handling

**Edge Cases To Consider:**
- [ ] Invalid mode flag
- [ ] Missing prompt
- [ ] Conflicting flags
- [ ] Mode transitions

## Success Metrics
- Command parsing accuracy: >99%
- Error recovery rate: >95%
- User satisfaction: >4.5/5
- Response time: <2s

## Related Tests
- [cli-version](#)
- [slash-commands](#)

## Changelog
- **2025-01-01**: Initial creation