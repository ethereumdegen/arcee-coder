---
name: slash-commands
tags: [cli, repl, commands]
skills: [json-api]
author: Arcee Coder Team
created: 2025-01-01
status: active
priority: high
---

## Description
Test all available slash commands in the Arcee Code REPL interface, ensuring proper functionality, error handling, and user experience.

## Test Scenarios

### Scenario 1: Help Command
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
    "command": "help",
    "repl_mode": true,
    "help_sections": ["available_commands", "descriptions", "examples"],
    "exit_code": 0
  }
}
```

**Validation Criteria:**
- [ ] All commands listed with descriptions
- [ ] Formatting consistent and readable
- [ ] No syntax errors
- [ ] Exit code 0
- [ ] REPL mode maintained
- [ ] Help text complete

**Edge Cases To Consider:**
- [ ] Unknown slash command
- [ ] Command with invalid arguments
- [ ] Help text truncation
- [ ] Terminal width limitations
- [ ] Color/syntax highlighting

### Scenario 2: Clear Command
**Input:**
```json
{
  "prompt": "/clear",
  "context": {
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
    "conversation_history": [],
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

**Edge Cases To Consider:**
- [ ] Already empty history
- [ ] Large conversation history
- [ ] Concurrent access to history
- [ ] Memory cleanup

### Scenario 3: Compact Command
**Input:**
```json
{
  "prompt": "/compact",
  "context": {
    "conversation_history": [
      {"user": "Message 1", "assistant": "Response 1"},
      {"user": "Message 2", "assistant": "Response 2"},
      {"user": "Message 3", "assistant": "Response 3"},
      {"user": "Message 4", "assistant": "Response 4"},
      {"user": "Message 5", "assistant": "Response 5"}
    ]
  },
  "options": {
    "repl_mode": true,
    "compression_level": "medium"
  }
}
```

**Expected Output:**
```json
{
  "response": "Conversation compacted from 5 messages to 3 messages",
  "metadata": {
    "command": "compact",
    "compression_applied": "medium",
    "messages_removed": 2,
    "context_size": "reduced",
    "exit_code": 0
  }
}
```

**Validation Criteria:**
- [ ] Context properly compressed
- [ ] Key information preserved
- [ ] No data loss
- [ ] Performance improved
- [ ] Exit code 0
- [ ] REPL mode maintained

**Edge Cases To Consider:**
- [ ] Already compact history
- [ ] Maximum compression level
- [ ] Different compression algorithms
- [ ] Memory usage during compression
- [ ] Large conversation history

### Scenario 4: Cost Command
**Input:**
```json
{
  "prompt": "/cost",
  "context": {
    "conversation_history": [
      {"user": "First message", "assistant": "First response (150 tokens)"},
      {"user": "Second message", "assistant": "Second response (200 tokens)"}
    ],
    "token_counts": [150, 200],
    "model": "trinity-large-thinking"
  },
  "options": {
    "repl_mode": true
  }
}
```

**Expected Output:**
```json
{
  "response": "Current session cost: $0.045\nToken usage: 350 tokens\nModel: trinity-large-thinking\nEstimated completion: 85%",
  "metadata": {
    "command": "cost",
    "token_count": 350,
    "estimated_cost": 0.045,
    "model_used": "trinity-large-thinking",
    "completion_percentage": 85,
    "exit_code": 0
  }
}
```

**Validation Criteria:**
- [ ] Token counts accurate
- [ ] Cost calculation correct
- [ ] Model information displayed
- [ ] Progress estimation reasonable
- [ ] Exit code 0
- [ ] REPL mode maintained

**Edge Cases To Consider:**
- [ ] Zero token usage
- [ ] Very large token counts
- [ ] Different pricing models
- [ ] Currency conversion
- [ ] Budget warnings

### Scenario 5: Model Command
**Input:**
```json
{
  "prompt": "/model",
  "context": {
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
  "response": "Current model: trinity-large-thinking\nAvailable models: trinity-large-thinking, trinity-medium-thinking, trinity-small-thinking",
  "metadata": {
    "command": "model",
    "current_model": "trinity-large-thinking",
    "available_models": ["trinity-large-thinking", "trinity-medium-thinking", "trinity-small-thinking"],
    "exit_code": 0
  }
}
```

**Validation Criteria:**
- [ ] Current model displayed correctly
- [ ] All available models listed
- [ ] Model switching supported
- [ ] Validation of model names
- [ ] Exit code 0
- [ ] REPL mode maintained

**Edge Cases To Consider:**
- [ ] Invalid model name
- [ ] Model not available
- [ ] Model loading errors
- [ ] Model-specific limitations

### Scenario 6: Intensity Command
**Input:**
```json
{
  "prompt": "/intensity high",
  "context": {
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
- [ ] REPL mode maintained

**Edge Cases To Consider:**
- [ ] Invalid intensity value
- [ ] Intensity already set
- [ ] Performance impact
- [ ] Model compatibility

### Scenario 7: Strictness Command
**Input:**
```json
{
  "prompt": "/strictness high",
  "context": {
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
- [ ] REPL mode maintained

**Edge Cases To Consider:**
- [ ] Invalid strictness value
- [ ] Strictness already set
- [ ] Permission changes
- [ ] Security implications

### Scenario 8: Tokens Command
**Input:**
```json
{
  "prompt": "/tokens",
  "context": {
    "conversation_history": [
      {"user": "First message", "assistant": "First response (150 tokens)"},
      {"user": "Second message", "assistant": "Second response (200 tokens)"}
    ],
    "token_counts": [150, 200],
    "model": "trinity-large-thinking"
  },
  "options": {
    "repl_mode": true
  }
}
```

**Expected Output:**
```json
{
  "response": "Estimated tokens: 350\nRemaining budget: 9,650",
  "metadata": {
    "command": "tokens",
    "estimated_tokens": 350,
    "remaining_budget": 9650,
    "token_limit": 10000,
    "exit_code": 0
  }
}
```

**Validation Criteria:**
- [ ] Token estimation accurate
- [ ] Budget calculation correct
- [ ] Clear presentation
- [ ] Exit code 0
- [ ] REPL mode maintained

**Edge Cases To Consider:**
- [ ] Zero remaining budget
- [ ] Negative token estimates
- [ ] Very large token counts
- [ ] Budget warnings

### Scenario 9: History Command
**Input:**
```json
{
  "prompt": "/history",
  "context": {
    "conversation_history": [
      {"user": "First message", "assistant": "First response"},
      {"user": "Second message", "assistant": "Second response"},
      {"user": "Third message", "assistant": "Third response"}
    ],
    "session_metadata": {
      "start_time": "2025-01-01T10:00:00Z",
      "model_used": "trinity-large-thinking",
      "total_tokens": 450
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
  "response": "Session History (3 messages):\n1. User: First message | Assistant: First response\n2. User: Second message | Assistant: Second response\n3. User: Third message | Assistant: Third response\n\nSession Info:\n- Start time: 2025-01-01 10:00:00 UTC\n- Model: trinity-large-thinking\n- Total tokens: 450",
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
- [ ] REPL mode maintained

**Edge Cases To Consider:**
- [ ] Empty history
- [ ] Very large history
- [ ] History with errors
- [ ] Session metadata missing

### Scenario 10: Quit Command
**Input:**
```json
{
  "prompt": "/quit",
  "context": {
    "repl_mode": true,
    "session_active": true
  },
  "options": {
    "repl_mode": true
  }
}
```

**Expected Output:**
```json
{
  "response": "Goodbye! Session ended.",
  "metadata": {
    "command": "quit",
    "session_ended": true,
    "repl_mode": false,
    "exit_code": 0
  }
}
```

**Validation Criteria:**
- [ ] Session properly terminated
- [ ] REPL mode exited
- [ ] Goodbye message displayed
- [ ] Resources cleaned up
- [ ] Exit code 0

**Edge Cases To Consider:**
- [ ] Multiple quit attempts
- [ ] Unsaved changes
- [ ] Active operations
- [ ] Graceful shutdown

## Success Metrics
- Command success rate: >99%
- Response time: <1s
- User satisfaction: >4.5/5
- Error recovery: >95%
- Accessibility: Screen reader compatible

## Related Tests
- [command-parsing](#)
- [ui-interaction](#)

## Changelog
- **2025-01-01**: Initial creation