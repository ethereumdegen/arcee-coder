---
name: config-management
tags: [config, settings]
skills: [json-api]
expected: Properly loads and applies configuration settings
---

Test the configuration system and its various sources.

The system should:
- Load default configuration from built-in defaults
- Read user configuration from ~/.config/arcee-code/config.toml
- Override settings with command-line flags
- Handle missing or malformed config files gracefully
- Persist user preferences across sessions
- Validate configuration values and provide helpful errors