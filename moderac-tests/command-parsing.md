---
name: command-parsing
tags: [cli, parsing]
skills: [json-api]
expected: Correctly parses and executes various CLI commands
---

Test various CLI commands and their parsing behavior.

The system should:
- Parse `arcee --help` and display help information
- Parse `arcee --version` and display version
- Parse `arcee "write code for X"` and start a coding session
- Parse `arcee --mode thinking "explain Y"` and activate thinking mode
- Handle invalid commands gracefully with helpful error messages