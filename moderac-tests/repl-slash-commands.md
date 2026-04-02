---
name: repl-slash-commands
tags: [repl, slash-commands, ui, interaction]
skills: [json-api]
author: Arcee Coder Team
created: 2025-01-01
status: active
priority: high
---

## Description
Test the REPL slash command functionality, including command parsing, execution, error handling, and user experience.

## Test Scenarios

### Scenario 1: Basic Slash Command Execution
**Input:**
```json
{
  "prompt": "/help",
  "context": {
    "repl_mode": true,
    "conversation_history": []
  },
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
    "command": "help",
    "repl_mode": true,
    "exit_code": 0,
    "help_sections": ["available_commands", "descriptions", "examples"]
  }
}
```
**Validation Criteria:**
- [ ] Command recognized
- [ ] Help text complete
- [ ] Formatting correct
- [ ] REPL mode maintained
- [ ] No syntax errors

### Scenario 2: Clear Command with History
**Input:**
```json
{
  "prompt": "/clear",
  "context": {
    "repl_mode": true,
    "conversation_history": [
      {"user": "First message", "assistant": "First response"},
      {"user": "Second message", "assistant": "Second response"}
    ]
  },
  "options": {
    "repl_mode": true
  }
}
```
**Expected Output:**
```json
{
  "response": "Conversation history cleared",
  "metadata": {
    "command": "clear",
    "history_cleared": true,
    "new_history_count": 0,
    "exit_code": 0
  }
}
```
**Validation Criteria:**
- [ ] History completely cleared
- [ ] Confirmation message displayed
- [ ] REPL mode maintained
- [ ] No residual data
- [ ] Exit code 0

### Scenario 3: Compact Command with Different Levels
**Input:**
```json
{
  "prompt": "/compact medium",
  "context": {
    "repl_mode": true,
    "conversation_history": [
      {"user": "Message 1", "assistant": "Response 1"},
      {"user": "Message 2", "assistant": "Response 2"},
      {"user": "Message 3", "assistant": "Response 3"},
      {"user": "Message 4", "assistant": "Response 4"},
      {"user": "Message 5", "assistant": "Response 5"},
      {"user": "Message 6", "assistant": "Response 6"}
    ]
  },
  "options": {
    "repl_mode": true
  }
}
```
**Expected Output:**
```json
{
  "response": "Conversation compacted from 6 messages to 4 messages",
  "metadata": {
    "command": "compact",
    "compression_level": "medium",
    "messages_removed": 2,
    "context_size_reduction": "33%",
    "exit_code": 0
  }
}
```
**Validation Criteria:**
- [ ] Command parsed with argument
- [ ] Context properly compressed
- [ ] Key information preserved
- [ ] Performance improved
- [ ] Exit code 0

### Scenario 4: Cost Command with Budget Tracking
**Input:**
```json
{
  "prompt": "/cost",
  "context": {
    "repl_mode": true,
    "conversation_history": [
      {"user": "Write Rust function", "assistant": "fn main() {} (150 tokens)"},
      {"user": "Explain code", "assistant": "Explanation (200 tokens)"}
    ],
    "token_counts": [150, 200],
    "model": "trinity-large-thinking",
    "budget_usd": 10.0,
    "cost_used": 0.045
  },
  "options": {
    "repl_mode": true
  }
}
```
**Expected Output:**
```json
{
  "response": "Current session cost: $0.045\nToken usage: 350 tokens\nModel: trinity-large-thinking\nEstimated completion: 85%\nRemaining budget: $9.955",
  "metadata": {
    "command": "cost",
    "token_count": 350,
    "estimated_cost": 0.045,
    "model_used": "trinity-large-thinking",
    "completion_percentage": 85,
    "remaining_budget": 9.955,
    "exit_code": 0
  }
}
```
**Validation Criteria:**
- [ ] Token counts accurate
- [ ] Cost calculation correct
- [ ] Budget tracking working
- [ ] Model information displayed
- [ ] Exit code 0

### Scenario 5: Model Command with Switching
**Input:**
```json
{
  "prompt": "/model trinity-medium-thinking",
  "context": {
    "repl_mode": true,
    "current_model": "trinity-large-thinking",
    "available_models": ["trinity-large-thinking", "trinity-medium-thinking", "trinity-small-thinking"]
  },
  "options": {
    "repl_mode": true
  }
}
```
**Expected Output:**
```json
{
  "response": "Model switched to: trinity-medium-thinking",
  "metadata": {
    "command": "model",
    "new_model": "trinity-medium-thinking",
    "previous_model": "trinity-large-thinking",
    "available_models": ["trinity-large-thinking", "trinity-medium-thinking", "trinity-small-thinking"],
    "exit_code": 0
  }
}
```
**Validation Criteria:**
- [ ] Command parsed with argument
- [ ] Model switched successfully
- [ ] Validation of model name
- [ ] REPL mode maintained
- [ ] Exit code 0

### Scenario 6: Intensity Command with Different Values
**Input:**
```json
{
  "prompt": "/intensity high",
  "context": {
    "repl_mode": true,
    "current_intensity": "medium",
    "available_intensities": ["high", "medium", "low"]
  },
  "options": {
    "repl_mode": true
  }
}
```
**Expected Output:**
```json
{
  "response": "Routing intensity set to: high",
  "metadata": {
    "command": "intensity",
    "new_intensity": "high",
    "previous_intensity": "medium",
    "exit_code": 0
  }
}
```
**Validation Criteria:**
- [ ] Intensity level changed correctly
- [ ] Validation of intensity values
- [ ] Proper error messages for invalid values
- [ ] Current intensity displayed when called without arguments
- [ ] Exit code 0

### Scenario 7: Strictness Command with Validation
**Input:**
```json
{
  "prompt": "/strictness high",
  "context": {
    "repl_mode": true,
    "current_strictness": "medium",
    "available_strictness": ["high", "medium", "low"]
  },
  "options": {
    "repl_mode": true
  }
}
```
**Expected Output:**
```json
{
  "response": "Permission strictness set to: high",
  "metadata": {
    "command": "strictness",
    "new_strictness": "high",
    "previous_strictness": "medium",
    "exit_code": 0
  }
}
```
**Validation Criteria:**
- [ ] Strictness level changed correctly
- [ ] Validation of strictness values
- [ ] Proper error messages for invalid values
- [ ] Current strictness displayed when called without arguments
- [ ] Exit code 0

### Scenario 8: Tokens Command with Budget Warnings
**Input:**
```json
{
  "prompt": "/tokens",
  "context": {
    "repl_mode": true,
    "conversation_history": [
      {"user": "Write complex code", "assistant": "Complex implementation (500 tokens)"},
      {"user": "Explain algorithm", "assistant": "Detailed explanation (800 tokens)"}
    ],
    "token_counts": [500, 800],
    "model": "trinity-large-thinking",
    "budget_usd": 5.0,
    "cost_used": 4.2
  },
  "options": {
    "repl_mode": true
  }
}
```
**Expected Output:**
```json
{
  "response": "Estimated tokens: 1300\nRemaining budget: $0.80 (4.2/5.0 spent)\nToken limit: 10000\n⚠️  Budget warning: 84% used",
  "metadata": {
    "command": "tokens",
    "estimated_tokens": 1300,
    "remaining_budget": 0.8,
    "token_limit": 10000,
    "budget_warning": true,
    "exit_code": 0
  }
}
```
**Validation Criteria:**
- [ ] Token estimation accurate
- [ ] Budget calculation correct
- [ ] Warning shown when budget >80%
- [ ] Clear presentation
- [ ] Exit code 0

### Scenario 9: History Command with Session Info
**Input:**
```json
{
  "prompt": "/history",
  "context": {
    "repl_mode": true,
    "conversation_history": [
      {"user": "First message", "assistant": "First response"},
      {"user": "Second message", "assistant": "Second response"},
      {"user": "Third message", "assistant": "Third response"}
    ],
    "session_metadata": {
      "start_time": "2025-01-01T10:00:00Z",
      "model_used": "trinity-large-thinking",
      "total_tokens": 450,
      "turn_count": 3
    }
  },
  "options": {
    "repl_mode": true
  }
}
```
**Expected Output:**
```json
{
  "response": "Session History (3 messages):\n1. User: First message | Assistant: First response\n2. User: Second message | Assistant: Second response\n3. User: Third message | Assistant: Third response\n\nSession Info:\n- Start time: 2025-01-01 10:00:00 UTC\n- Model: trinity-large-thinking\n- Total tokens: 450\n- Current turn: 3\n- Max turns: 200",
  "metadata": {
    "command": "history",
    "history_summary": true,
    "session_info": true,
    "exit_code": 0
  }
}
```
**Validation Criteria:**
- [ ] History properly summarized
- [ ] Session information included
- [ ] Formatting clear and readable
- [ ] Exit code 0

### Scenario 10: Quit Command with Session Cleanup
**Input:**
```json
{
  "prompt": "/quit",
  "context": {
    "repl_mode": true,
    "session_active": true,
    "conversation_history": [
      {"user": "Hello", "assistant": "Hi there!"}
    ]
  },
  "options": {
    "repl_mode": true
  }
}
```
**Expected Output:**
```json
{
  "response": "Goodbye! Session ended. History saved to ~/.arcee-code/sessions/session_123.json",
  "metadata": {
    "command": "quit",
    "session_ended": true,
    "repl_mode": false,
    "session_saved": true,
    "exit_code": 0
  }
}
```
**Validation Criteria:**
- [ ] Session properly terminated
- [ ] REPL mode exited
- [ ] Goodbye message displayed
- [ ] Session saved correctly
- [ ] Resources cleaned up
- [ ] Exit code 0

### Edge Cases
- [ ] Unknown slash command
- [ ] Command with invalid arguments
- [ ] Help text truncation
- [ ] Terminal width limitations
- [ ] Color/syntax highlighting
- [ ] Multiple commands in one line
- [ ] Command history navigation
- [ ] Command execution errors
- [ ] Concurrent command execution
- [ ] Command with special characters

### Success Metrics
- Command success rate: >99%
- Response time: <500ms
- User satisfaction: >4.5/5
- Error recovery: >95%
- Accessibility: Screen reader compatible

### Related Tests
- [command-parsing](#)
- [ui-interaction](#)

### Changelog
- **2025-01-01**: Initial creation
- **2025-01-02**: Added more command scenarios
- **2025-01-03**: Added edge case testing