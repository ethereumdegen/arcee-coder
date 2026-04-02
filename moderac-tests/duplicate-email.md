---
name: duplicate-email
tags: [auth, error, validation]
skills: [json-api]
expected: Returns appropriate error for duplicate email with clear message
---

Test the duplicate email handling during user signup.

The system should:
- Detect when an email address is already registered
- Return a clear error indicating the email is taken
- Include appropriate HTTP status code (409 Conflict)
- Provide a helpful error message for user feedback
- Not create a duplicate user account
- Handle multiple concurrent signup attempts gracefully
- Log the duplicate attempt for security monitoring
- Suggest password recovery if user exists