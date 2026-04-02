---
name: json-api
description: Skill for testing JSON REST APIs
---

You are testing a JSON REST API. All requests and responses use JSON.
When evaluating responses, check for:
- Correct HTTP status codes (200, 201, 400, 401, 403, 404, 409, 500, etc.)
- Valid JSON structure and syntax
- Required fields present in response body
- Appropriate error messages for failure cases
- Proper content-type headers (application/json)
- Response time performance (under acceptable thresholds)
- Rate limiting headers and behavior
- Authentication headers and token handling
- Pagination support when applicable
- Data validation and sanitization
- CORS headers when appropriate
- API versioning support
- Rate limit headers (X-RateLimit-Limit, X-RateLimit-Remaining)