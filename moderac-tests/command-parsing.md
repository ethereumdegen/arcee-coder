---
name: command-parsing
tags: [cli, parsing, arguments]
skills: [json-api]
expected: Correctly parses and executes various CLI commands and REPL commands
---

Test various CLI commands and their parsing behavior, including both command-line flags and REPL slash commands.

The system should:
- Parse `arcee --help` and display help information
- Parse `arcee --version` and display version
- Parse `arcee "write code for X"` and start a coding session
- Parse `arcee --mode thinking "explain Y"` and activate thinking mode
- Parse `arcee --mode ui "create UI for Z"` and activate UI mode
- Parse `arcee --skill bash "run shell command"` and use the bash skill
- Parse `arcee --skill web_search "search the web"` and use the web search skill
- Parse `arcee --skill lsp "find definitions"` and use LSP skills
- Parse `arcee --skill skill "run skill command"` and use custom skills
- Handle invalid commands gracefully with helpful error messages
- Support quoted arguments with spaces
- Handle special characters in commands
- Provide autocomplete suggestions for commands
- Support REPL slash commands: `/help`, `/clear`, `/compact`, `/cost`, `/model`, `/intensity`, `/strictness`, `/tokens`, `/history`, `/quit`
- Execute REPL commands correctly with proper output
- Handle REPL command history navigation
- Support command-line flags and REPL commands in combination