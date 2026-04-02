---
description: Review code for simplification opportunities
---
Review the specified code for simplification:

1. Read the target file(s)
2. Identify:
   - Over-abstracted code that could be inlined
   - Unnecessary wrapper types or helper functions
   - Redundant error handling or validation
   - Dead code or unused imports
   - Complex control flow that could be simplified
   - Premature optimizations
3. For each finding:
   - Explain why it's over-complex
   - Show the simplified version
   - Confirm the simplification preserves behavior
4. Apply changes if the user approves
