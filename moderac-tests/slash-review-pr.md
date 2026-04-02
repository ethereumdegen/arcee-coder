---
name: slash-review-pr
tags: [cli, pr, review, repl]
skills: [json-api]
expected: Reviews a pull request for code quality and correctness via slash command
---

Test the `/review-pr` slash command for pull request review.

The system should:
- Accept PR number or use current PR
- Get PR details: `gh pr view [number] --json title,body,files,additions,deletions`
- Get the diff: `gh pr diff [number]`
- Review each changed file for:
  - Correctness: Logic errors, edge cases, off-by-one errors
  - Security: Input validation, injection vulnerabilities, auth issues
  - Performance: N+1 queries, unnecessary allocations, missing indexes
  - Style: Naming conventions, code organization, consistency
  - Tests: Adequate coverage, edge cases tested
- Check for:
  - Breaking changes without migration
  - Missing error handling
  - Incomplete implementations (TODO/FIXME)
- Provide a structured review with:
  - Summary of changes
  - Issues found (critical, minor, nit)
  - Suggestions for improvement
  - Overall assessment (approve/request changes)
- Handle missing GitHub CLI gracefully
- Handle invalid PR numbers appropriately
- Provide clear, actionable feedback