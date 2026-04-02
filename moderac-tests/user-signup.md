---
name: user-signup
tags: [auth, signup, validation]
skills: [json-api]
expected: Successfully creates user accounts with proper validation and email verification
---

Test the user signup functionality.

The system should:
- Create the user account with valid email and password
- Return the new user's ID in the response
- Send a welcome email to the provided address
- Validate email format and password strength
- Hash passwords securely before storage
- Handle duplicate email addresses appropriately
- Support email verification flows
- Handle invalid input gracefully with clear error messages
- Log signup events for analytics and security
- Support social login integrations
- Handle account activation and confirmation