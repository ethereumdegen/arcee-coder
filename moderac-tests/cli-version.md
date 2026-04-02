---
name: cli-version
tags: [cli, version, help]
skills: [json-api]
expected: Correctly displays version and help information
---

Test the CLI version and help commands.

The system should:
- Display the version number when running `arcee --version`
- Show the version in format "arcee-code 2.1.2" matching Cargo.toml
- Display comprehensive help information with `arcee --help`
- Show usage examples in help text
- Display all available commands and their descriptions
- Show proper formatting and readability in help output
- Handle invalid flags gracefully with helpful error messages