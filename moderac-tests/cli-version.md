---
name: cli-version
tags: [cli, version, help]
skills: [json-api]
author: Arcee Coder Team
created: 2025-01-01
status: active
priority: high
---

## Description
Test the CLI version and help commands to ensure proper display of version information and comprehensive help documentation.

## Test Scenarios

### Scenario 1: Display Version Information
**Input:**
```json
{
  "prompt": "arcee --version",
  "context": {},
  "options": {}
}
```

**Expected Output:**
```json
{
  "response": "arcee-code 2.1.2",
  "metadata": {
    "exit_code": 0
  }
}
```

**Validation Criteria:**
- [ ] Version format matches "arcee-code X.Y.Z"
- [ ] Version number corresponds to Cargo.toml
- [ ] Exit code is 0
- [ ] Output is clean and readable
- [ ] No error messages

**Edge Cases To Consider:**
- [ ] Invalid flags
- [ ] Missing Cargo.toml
- [ ] Version parsing errors

### Scenario 2: Display Help Information
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
  "response": "Usage: arcee [OPTIONS] [prompt]\n\nArguments:\n    prompt    The prompt to send to the AI\n\nOptions:\n    -h, --help       Print help\n    -V, --version    Print version\n    --model <model>  Specify the model to use\n    --budget <n>     Maximum tokens to use\n    --mode <mode>    Operation mode (thinking, ui, normal)\n    --skill <skill>  Use a specific skill\n    --json           Output in JSON format\n    --debug          Enable debug mode\n\nExamples:\n    arcee \"Write a Rust function\"\n    arcee --model \"trinity-large-thinking\" \"Explain this code\"\n    arcee --budget 5.0 \"Write a complex algorithm\"\n    arcee --mode thinking \"Analyze this code\"",
  "metadata": {
    "exit_code": 0,
    "help_sections": ["usage", "arguments", "options", "examples"]
  }
}
```

**Validation Criteria:**
- [ ] All command-line options documented
- [ ] Usage examples provided
- [ ] Formatting is consistent
- [ ] No syntax errors in help text
- [ ] Exit code is 0

**Edge Cases To Consider:**
- [ ] Missing required arguments
- [ ] Invalid option combinations
- [ ] Help text truncation

## Success Metrics
- Help text completeness: 100%
- Version accuracy: 100%
- User satisfaction: >4.5/5

## Related Tests
- [command-parsing](#)
- [slash-commands](#)

## Changelog
- **2025-01-01**: Initial creation