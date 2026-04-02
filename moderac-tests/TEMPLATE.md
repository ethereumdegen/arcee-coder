---
name: test-name
tags: [category, feature, priority]
skills: [skill1, skill2]
author: your-name
created: YYYY-MM-DD
status: active | deprecated | wip
priority: high | medium | low
---

## Description
[Detailed description of what this test validates, including the purpose, scope, and expected outcomes]

## Test Scenarios

### Scenario 1: [Scenario Name]
**Input:**
```json
{
  "prompt": "...",
  "context": {...},
  "options": {...}
}
```

**Expected Output:**
```json
{
  "response": "...",
  "changes": [...],
  "diff": "...",
  "metadata": {...}
}
```

**Validation Criteria:**
- [ ] Response structure matches expected schema
- [ ] All required fields present
- [ ] Code changes are syntactically correct
- [ ] Behavior preservation verified
- [ ] Error handling appropriate
- [ ] Performance within acceptable bounds
- [ ] Security considerations addressed

**Edge Cases to Consider:**
- [ ] Empty input
- [ ] Invalid syntax
- [ ] Large file handling
- [ ] Permission issues
- [ ] Network failures

### Scenario 2: [Scenario Name]
...

## Success Metrics
- Code quality improvements: [measure]
- Performance impact: [measure]
- User experience: [measure]
- Error rate: [measure]

## Related Tests
- [Related Test 1](#)
- [Related Test 2](#)

## Changelog
- **YYYY-MM-DD**: Initial creation
- **YYYY-MM-DD**: [Update description]