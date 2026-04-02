---
name: file-operations
tags: [file, io, safety]
skills: [json-api]
expected: Safely reads and writes files with proper error handling
---

Test file reading and writing operations performed by the CLI.

The system should:
- Read files safely with proper path validation
- Write files to user-specified locations
- Handle permission errors gracefully
- Prevent directory traversal attacks
- Create parent directories when needed
- Handle large file operations efficiently