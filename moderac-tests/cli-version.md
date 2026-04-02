---
name: cli-version
tags: [cli, version]
skills: [json-api]
expected: Returns the current version string matching Cargo.toml
---

Run the arcee CLI with --version flag and capture the output.

The system should:
- Display the version number
- The version should match the Cargo.toml version (2.0.1)
- Format: "arcee-code 2.0.1"