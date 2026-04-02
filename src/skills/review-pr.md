---
description: Review a pull request for code quality and correctness
---
Review the current PR or specified PR number:

1. Get PR details: `gh pr view [number] --json title,body,files,additions,deletions`
2. Get the diff: `gh pr diff [number]`
3. Review each changed file for:
   - Correctness: Logic errors, edge cases, off-by-one errors
   - Security: Input validation, injection vulnerabilities, auth issues
   - Performance: N+1 queries, unnecessary allocations, missing indexes
   - Style: Naming conventions, code organization, consistency
   - Tests: Adequate coverage, edge cases tested
4. Check for:
   - Breaking changes without migration
   - Missing error handling
   - Incomplete implementations (TODO/FIXME)
5. Provide a structured review with:
   - Summary of changes
   - Issues found (critical, minor, nit)
   - Suggestions for improvement
   - Overall assessment (approve/request changes)
