---
name: file-operations
tags: [file, io, safety, permissions]
skills: [json-api]
expected: Safely reads and writes files with proper error handling and security
---

Test file reading and writing operations performed by the CLI.

The system should:
- Read files safely with proper path validation
- Write files to user-specified locations
- Handle permission errors gracefully
- Prevent directory traversal attacks (sanitize paths)
- Create parent directories when needed
- Handle large file operations efficiently (streaming)
- Support atomic file writes to prevent corruption
- Handle file locking and concurrent access
- Validate file content before writing
- Support temporary file operations
- Handle read-only file systems appropriately
- Provide progress indicators for large file operations