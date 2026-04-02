---
description: Create a git commit with a well-crafted message
---
Follow these steps to create a git commit:

1. Run `git status` to see all changes (never use -uall flag)
2. Run `git diff` to see staged and unstaged changes
3. Run `git log --oneline -5` to see recent commit message style
4. Analyze the changes and draft a concise commit message:
   - Summarize the nature (new feature, bug fix, refactor, etc.)
   - Focus on "why" not "what"
   - Keep to 1-2 sentences
5. Stage relevant files (prefer specific files over `git add -A`)
6. Create the commit using a HEREDOC for the message
7. Run `git status` to verify success

Do NOT push unless explicitly asked. Do NOT amend existing commits unless asked.
