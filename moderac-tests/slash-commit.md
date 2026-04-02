---
name: slash-commit
tags: [cli, git, commit, repl]
skills: [json-api]
expected: Creates a git commit with a well-crafted message via slash command
---

Test the `/commit` slash command for creating Git commits.

The system should:
- Accept a commit message via the slash command
- Run `git status` to see all changes (never use -uall flag)
- Run `git diff` to see staged and unstaged changes
- Run `git log --oneline -5` to see recent commit message style
- Analyze the changes and draft a concise commit message
- Stage relevant files (prefer specific files over `git add -A`)
- Create the commit using a HEREDOC for the message
- Run `git status` to verify success
- NOT push unless explicitly asked
- NOT amend existing commits unless asked
- Handle missing Git repository gracefully
- Handle unstaged changes appropriately
- Provide clear feedback throughout the process