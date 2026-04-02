---
name: api-integration
tags: [api, http, integration, ai]
skills: [json-api]
expected: Successfully integrates with AI API and handles responses correctly
---

Test the API client integration with the AI service.

The system should:
- Connect to the configured AI API endpoint
- Handle authentication properly (API keys, tokens)
- Stream responses correctly with proper error handling
- Handle API errors gracefully (timeouts, rate limits, invalid responses)
- Retry failed requests when appropriate (exponential backoff)
- Parse JSON responses accurately
- Handle large responses efficiently
- Support streaming responses with proper chunking
- Handle network interruptions and resume gracefully
- Validate API responses against expected schemas