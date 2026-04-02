---
name: config-management
tags: [config, settings, defaults]
skills: [json-api]
author: Arcee Coder Team
created: 2025-01-01
status: active
priority: high
---

## Description
Test the configuration system and its various sources, ensuring proper loading, merging, and application of configuration settings from defaults, config files, and command-line flags.

## Test Scenarios

### Scenario 1: Load Default Configuration
**Input:**
```json
{
  "prompt": "Get current configuration",
  "context": {},
  "options": {}
}
```

**Expected Output:**
```json
{
  "response": "{\n  \"api_key\": \"\",\n  \"model\": \"trinity-large-thinking\",\n  \"budget\": 10.0,\n  \"timeout\": 30,\n  \"strictness\": \"medium\",\n  \"intensity\": \"medium\",\n  \"stream\": true,\n  \"debug\": false,\n  \"color\": true\n}",
  "metadata": {
    "config_loaded": true,
    "source": "defaults",
    "values": {
      "api_key": "",
      "model": "trinity-large-thinking",
      "budget": 10.0,
      "timeout": 30,
      "strictness": "medium",
      "intensity": "medium",
      "stream": true,
      "debug": false,
      "color": true
    }
  }
}
```

**Validation Criteria:**
- [ ] All default values loaded correctly
- [ ] No API key present by default
- [ ] Proper data types for each config value
- [ ] Config validation passes
- [ ] Missing config file handled gracefully

**Edge Cases To Consider:**
- [ ] Corrupted config file
- [ ] Missing config directory
- [ ] Invalid config values
- [ ] Permission denied on config file
- [ ] Config file with syntax errors

### Scenario 2: Override with User Config
**Input:**
```json
{
  "prompt": "Load configuration from ~/.config/arcee-code/config.toml",
  "context": {},
  "options": {
    "config_path": "/home/test/.config/arcee-code/config.toml"
  }
}
```

**Expected Output:**
```json
{
  "response": "Configuration loaded from /home/test/.config/arcee-code/config.toml",
  "metadata": {
    "config_loaded": true,
    "source": "user_config",
    "values": {
      "api_key": "test-api-key-123",
      "model": "trinity-large-thinking",
      "budget": 20.0,
      "timeout": 60,
      "strictness": "high",
      "intensity": "high",
      "stream": false,
      "debug": true,
      "color": false
    }
  }
}
```

**Validation Criteria:**
- [ ] User config file found and parsed
- [ ] Values correctly override defaults
- [ ] Proper error handling for missing file
- [ ] Config validation passes
- [ ] Changes applied without restart

**Edge Cases To Consider:**
- [ ] Config file with relative paths
- [ ] Config file with environment variables
- [ ] Config file with includes
- [ ] Config file with comments
- [ ] Config file with duplicate keys

### Scenario 3: Command-Line Flag Override
**Input:**
```json
{
  "prompt": "arcee --model \"trinity-medium-thinking\" --budget 5.0 --timeout 15 \"write code\"",
  "context": {},
  "options": {}
}
```

**Expected Output:**
```json
{
  "response": "Using model: trinity-medium-thinking\nBudget: 5.0 tokens\nTimeout: 15s\nCode generated successfully",
  "metadata": {
    "config_overridden": true,
    "values": {
      "model": "trinity-medium-thinking",
      "budget": 5.0,
      "timeout": 15
    },
    "prompt_processed": true
  }
}
```

**Validation Criteria:**
- [ ] Command-line flags parsed correctly
- [ ] Values override both defaults and user config
- [ ] Prompt processed with new settings
- [ ] Proper error messages for invalid flags
- [ ] Help text displayed for --help

**Edge Cases To Consider:**
- [ ] Invalid model name
- [ ] Negative budget
- [ ] Zero timeout
- [ ] Conflicting flags
- [ ] Unknown flags

### Scenario 4: Environment Variable Override
**Input:**
```json
{
  "prompt": "Set ARCEE_API_KEY=test-key-456 and get configuration",
  "context": {},
  "options": {
    "env": {
      "ARCEE_API_KEY": "test-key-456"
    }
  }
}
```

**Expected Output:**
```json
{
  "response": "API key loaded from environment variable",
  "metadata": {
    "config_overridden": true,
    "values": {
      "api_key": "test-key-456"
    },
    "env_variables": ["ARCEE_API_KEY"]
  }
}
```

**Validation Criteria:**
- [ ] Environment variables correctly detected
- [ ] Values override other config sources
- [ ] Proper error handling for missing variables
- [ ] Security: Sensitive data masked
- [ ] Multiple environment variables supported

**Edge Cases To Consider:**
- [ ] Invalid environment variable names
- [ ] Empty environment variable values
- [ ] Environment variable conflicts
- [ ] Security vulnerabilities

## Success Metrics
- Config loading success rate: >99%
- Merge order correctness: 100%
- Performance impact: <100ms
- Error recovery: >95%

## Related Tests
- [command-parsing](#)
- [api-integration](#)

## Changelog
- **2025-01-01**: Initial creation