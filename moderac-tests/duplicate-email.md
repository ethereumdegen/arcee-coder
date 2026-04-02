---
name: duplicate-email
tags: [auth, error, validation]
skills: [json-api]
author: Arcee Coder Team
created: 2025-01-01
status: active
priority: medium
---

## Description
Test the duplicate email handling during user signup, ensuring proper error detection, clear messaging, and appropriate HTTP status codes.

## Test Scenarios

### Scenario 1: Detect Duplicate Email
**Input:**
```json
{
  "prompt": "Signup user with email test@example.com and password 'SecurePass123!'",
  "context": {},
  "options": {
    "existing_users": [
      {"email": "test@example.com", "password_hash": "$2y$12$LQv3uR5P7Y2e6tH4rJ8k0."}
    ]
  }
}
```

**Expected Output:**
```json
{
  "response": "Error: Email address is already registered",
  "metadata": {
    "error": "duplicate_email",
    "status_code": 409,
    "user_message": "The email address test@example.com is already registered. Please use a different email or recover your password.",
    "help_link": "https://example.com/recover-password"
  }
}
```

**Validation Criteria:**
- [ ] Duplicate email correctly detected
- [ ] HTTP status code 409 (Conflict) returned
- [ ] Clear user-friendly error message
- [ ] No user account created
- [ ] Proper logging of the attempt
- [ ] Security: No user data exposed

**Edge Cases To Consider:**
- [ ] Case-insensitive email comparison
- [ ] Email with leading/trailing spaces
- [ ] Concurrent signup attempts
- [ ] Invalid email format
- [ ] Weak password detection
- [ ] SQL injection attempts in email field

### Scenario 2: Suggest Password Recovery
**Input:**
```json
{
  "prompt": "Signup with duplicate email but suggest recovery",
  "context": {},
  "options": {
    "existing_users": [
      {"email": "test@example.com", "password_hash": "$2y$12$LQv3uR5P7Y2e6tH4rJ8k0."}
    ],
    "suggest_recovery": true
  }
}
```

**Expected Output:**
```json
{
  "response": "Error: Email already registered. [Recover your account](https://example.com/recover-password?email=test@example.com)",
  "metadata": {
    "error": "duplicate_email",
    "status_code": 409,
    "recovery_suggested": true,
    "recovery_link": "https://example.com/recover-password?email=test@example.com"
  }
}
```

**Validation Criteria:**
- [ ] Recovery suggestion included
- [ ] Secure recovery link generated
- [ ] Email address pre-filled in link
- [ ] No password reset performed automatically
- [ ] User experience friendly

**Edge Cases To Consider:**
- [ ] User already has active recovery link
- [ ] Recovery link expiration
- [ ] Multiple recovery attempts
- [ ] Security: Link tampering prevention

### Scenario 3: Handle Multiple Concurrent Signups
**Input:**
```json
{
  "prompt": "Multiple concurrent signup attempts with same email",
  "context": {},
  "options": {
    "concurrent_attempts": 5,
    "existing_users": [
      {"email": "test@example.com", "password_hash": "$2y$12$LQv3uR5P7Y2e6tH4rJ8k0."}
    ]
  }
}
```

**Expected Output:**
```json
{
  "response": "All 5 signup attempts failed with duplicate email error",
  "metadata": {
    "attempts": 5,
    "failed_attempts": 5,
    "duplicate_errors": 5,
    "database_integrity": "maintained",
    "race_condition_prevention": true
  }
}
```

**Validation Criteria:**
- [ ] All attempts properly rejected
- [ ] No race conditions
- [ ] Database integrity maintained
- [ ] Proper error messages for all attempts
- [ ] Performance under load acceptable

**Edge Cases To Consider:**
- [ ] Database connection pooling
- [ ] Transaction isolation levels
- [ ] Lock contention
- [ ] Timeout handling
- [ ] Resource cleanup

## Success Metrics
- Duplicate detection accuracy: 100%
- Error message clarity: >4.5/5
- Concurrent signup handling: 100% rejection rate
- Performance under load: <1s per attempt
- Security: No data leakage

## Related Tests
- [user-signup](#)
- [api-integration](#)

## Changelog
- **2025-01-01**: Initial creation