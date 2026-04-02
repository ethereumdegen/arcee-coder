---
name: duplicate-email
tags: [auth, error]
skills: [json-api]
expected: Returns an appropriate error message about duplicate email
---

Attempt to sign up with an email address that is already registered in the system.

The system should reject the request and return a clear error indicating the email is taken.
