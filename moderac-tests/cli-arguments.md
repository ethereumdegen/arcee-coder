---
name: cli-arguments
tags: [cli, parsing, arguments, flags]
skills: [json-api]
author: Arcee Coder Team
created: 2025-01-01
status: active
priority: high
---

## Description
Test the command-line argument parsing and configuration handling, including both short and long flags, positional arguments, and environment variable overrides.

## Test Scenarios

### Scenario 1: Basic CLI Usage
**Input:**
```json
{
  "prompt": "arcee \"Write a Rust function\"",
  "context": {},
  "options": {}
}
```
**Expected Output:**
```json
{
  "response": "```rust\n// Generated code\n```",
  "metadata": {
    "session_id": "session_123",
    "prompt_analyzed": true,
    "code_generated": true,
    "model_used": "trinity-large-thinking"
  }
}
```
**Validation Criteria:**
- [ ] Prompt correctly extracted from quotes
- [ ] Session created successfully
- [ ] AI response generated
- [ ] Code syntax valid
- [ ] Exit code 0

### Scenario 2: Model Selection Flag
**Input:**
```json
{
  "prompt": "arcee --model \"trinity-medium-thinking\" \"Explain this code\"",
  "context": {},
  "options": {}
}
```
**Expected Output:**
```json
{
  "response": "Explanation of code...",
  "metadata": {
    "model_used": "trinity-medium-thinking",
    "budget_used": 0.02,
    "tokens_consumed": 1500
  }
}
```
**Validation Criteria:**
- [ ] Model flag parsed correctly
- [ ] Model switched to requested one
- [ ] Response generated with correct model
- [ ] Budget tracking accurate

### Scenario 3: Budget Limit
**Input:**
```json
{
  "prompt": "arcee --budget 1.0 \"Write complex algorithm\"",
  "context": {},
  "options": {}
}
```
**Expected Output:**
```json
{
  "response": "Algorithm implementation...",
  "metadata": {
    "budget_limit": 1.0,
    "cost_incurred": 0.05,
    "within_budget": true
  }
}
```
**Validation Criteria:**
- [ ] Budget flag parsed correctly
- [ ] Cost tracking active
- [ ] Response generated within budget
- [ ] Proper error if budget exceeded

### Scenario 4: Permission Modes
**Input:**
```json
{
  "prompt": "arcee --permission-mode auto \"Write to file\"",
  "context": {},
  "options": {}
}
```
**Expected Output:**
```json
{
  "response": "File written successfully",
  "metadata": {
    "permission_mode": "auto",
    "file_operation": "write",
    "security_check": "passed"
  }
}
```
**Validation Criteria:**
- [ ] Permission mode flag parsed
- [ ] Auto mode activated
- [ ] File operation performed
- [ ] Security checks passed

### Scenario 5: Help and Version
**Input:**
```json
{
  "prompt": "arcee --help",
  "context": {},
  "options": {}
}
```
**Expected Output:**
```json
{
  "response": "Usage: arcee [OPTIONS] [prompt]\n\nOptions:\n  -h, --help       Print help\n  -V, --version    Print version\n  --model <model>  Specify the model to use\n  ...\n\nExamples:\narcee \"Write a Rust function\"\narcee --model \"trinity-large-thinking\" \"Explain this code\"",
  "metadata": {
    "exit_code": 0,
    "help_sections": ["usage", "options", "examples"],
    "version_info": "arcee-code 2.1.2"
  }
}
```
**Validation Criteria:**
- [ ] Help text complete
- [ ] All options documented
- [ ] Version information correct
- [ ] Exit code 0

### Edge Cases
- [ ] Missing positional argument
- [ ] Invalid flag combination
- [ ] Unknown flags
- [ ] Empty prompt
- [ ] Special characters in prompt
- [ ] Very long prompts (>1000 chars)
- [ ] Conflicting flags
- [ ] Case-insensitive flag handling

### Success Metrics
- Argument parsing accuracy: >99%
- Error message clarity: >4.5/5
- Help text completeness: 100%
- Performance: <100ms for parsing

### Related Tests
- [command-parsing](#)
- [config-management](#)

### Changelog
- **2025-01-01**: Initial creation
- **2025-01-02**: Added permission mode testing