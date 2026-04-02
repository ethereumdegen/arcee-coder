---
name: user-signup
tags: [auth, signup]
skills: [json-api]
expected: Returns a success response with a user ID and sends a welcome email
---

Sign up a new user with email test@example.com and password "SecurePass123!".

The system should:
- Create the user account
- Return the new user's ID
- Send a welcome email to the provided address
