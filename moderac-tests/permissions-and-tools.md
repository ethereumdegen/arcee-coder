---
name: permissions-and-tools
tags: [permissions, security, tools, execution]
skills: [json-api]
author: Arcee Coder Team
created: 2025-01-01
status: active
priority: high
---

## Description
Test the permission system, tool execution, and security controls, including allow/deny rules, permission strictness, and tool validation.

## Test Scenarios

### Scenario 1: Default Permission Mode
**Input:**
```json
{
  "prompt": "Write code to /tmp/test.rs",
  "context": {},
  "options": {
    "repl_mode": true,
    "permission_mode": "default"
  }
}
```
**Expected Output:**
```json
{
  "response": "File written successfully",
  "metadata": {
    "permission_check": "passed",
    "tool_executed": "write",
    "security_context": "default"
  }
}
```
**Validation Criteria:**
- [ ] Default permission mode applied
- [ ] File write operation allowed
- [ ] Security checks passed
- [ ] No permission prompts

### Scenario 2: Strict Permission Mode (High)
**Input:**
```json
{
  "prompt": "Write to /etc/passwd",
  "context": {},
  "options": {
    "repl_mode": true,
    "permission_strictness": "high"
  }
}
```
**Expected Output:**
```json
{
  "response": "Error: Permission denied",
  "metadata": {
    "permission_result": "denied",
    "reason": "system_protected_path",
    "strictness_level": "high"
  }
}
```
**Validation Criteria:**
- [ ] High strictness applied
- [ ] System path protected
- [ ] Permission denied
- [ ] No user prompt

### Scenario 3: Medium Permission Mode (Ask)
**Input:**
```json
{
  "prompt": "Write to /home/user/project/src/main.rs",
  "context": {},
  "options": {
    "repl_mode": true,
    "permission_strictness": "medium"
  }
}
```
**Expected Output:**
```json
{
  "response": "Permission prompt shown to user",
  "metadata": {
    "permission_result": "prompted",
    "strictness_level": "medium",
    "user_decision": "allowed"
  }
}
```
**Validation Criteria:**
- [ ] Medium strictness applied
- [ ] Permission prompt shown
- [ ] User decision captured
- [ ] Operation allowed if user approves

### Scenario 4: Low Permission Mode (Allow)
**Input:**
```json
{
  "prompt": "Write to /tmp/test.rs",
  "context": {},
  "options": {
    "repl_mode": true,
    "permission_strictness": "low"
  }
}
```
**Expected Output:**
```json
{
  "response": "File written successfully",
  "metadata": {
    "permission_result": "allowed",
    "strictness_level": "low",
    "auto_allowed": true
  }
}
```
**Validation Criteria:**
- [ ] Low strictness applied
- [ ] Operation automatically allowed
- [ ] No security prompts
- [ ] Proper logging

### Scenario 5: Allow Rules Configuration
**Input:**
```json
{
  "prompt": "arcee --allow \"write:/tmp/*\" \"Write to /tmp/test.rs\"",
  "context": {},
  "options": {}
}
```
**Expected Output:**
```json
{
  "response": "File written successfully",
  "metadata": {
    "allow_rule": "write:/tmp/*",
    "operation": "write",
    "path_matched": "/tmp/test.rs",
    "allowed_by_rule": true
  }
}
```
**Validation Criteria:**
- [ ] Allow rule parsed correctly
- [ ] Path pattern matching works
- [ ] Operation allowed by rule
- [ ] Security context updated

### Scenario 6: Deny Rules Configuration
**Input:**
```json
{
  "prompt": "arcee --deny \"write:/etc/*\" \"Write to /etc/hosts\"",
  "context": {},
  "options": {}
}
```
**Expected Output:**
```json
{
  "response": "Error: Permission denied",
  "metadata": {
    "deny_rule": "write:/etc/*",
    "operation": "write",
    "path_matched": "/etc/hosts",
    "denied_by_rule": true
  }
}
```
**Validation Criteria:**
- [ ] Deny rule parsed correctly
- [ ] Path pattern matching works
- [ ] Operation denied by rule
- [ ] Proper error message

### Scenario 7: Tool Input Validation
**Input:**
```json
{
  "prompt": "Write code to file",
  "context": {},
  "options": {
    "repl_mode": true
  }
}
```
**Expected Output:**
```json
{
  "response": "Error: Missing required parameter 'file_path'",
  "metadata": {
    "validation_error": true,
    "missing_parameter": "file_path",
    "tool_name": "write",
    "schema_check": "failed"
  }
}
```
**Validation Criteria:**
- [ ] Required parameter detection
- [ ] Clear error message
- [ ] Schema validation working
- [ ] No tool execution

### Scenario 8: Tool Type Validation
**Input:**
```json
{
  "prompt": "Write code with invalid type",
  "context": {},
  "options": {
    "repl_mode": true,
    "tool_input": {
      "file_path": 123,
      "content": "fn main() {}"
    }
  }
}
```
**Expected Output:**
```json
{
  "response": "Error: Parameter 'file_path' must be string, got number",
  "metadata": {
    "validation_error": true,
    "parameter": "file_path",
    "expected_type": "string",
    "actual_type": "number",
    "tool_name": "write"
  }
}
```
**Validation Criteria:**
- [ ] Type checking working
- [ ] Clear error message
- [ ] Schema validation working
- [ ] No tool execution

### Edge Cases
- [ ] Empty tool call
- [ ] Unknown tool name
- [ ] Concurrent permission checks
- [ ] Permission rule conflicts
- [ ] Pattern matching edge cases
- [ ] Schema validation with nested objects
- [ ] Tool execution with partial parameters
- [ ] Permission mode switching during session
- [ ] Rule precedence (allow vs deny)
- [ ] Performance under load

### Success Metrics
- Permission accuracy: 100%
- Tool validation success rate: >99%
- Security vulnerabilities: 0
- Response time: <500ms
- User satisfaction: >4.5/5

### Related Tests
- [api-integration](#)
- [config-management](#)

### Changelog
- **2025-01-01**: Initial creation
- **2025-01-02**: Added tool validation scenarios
- **2025-01-03**: Added permission rule testing