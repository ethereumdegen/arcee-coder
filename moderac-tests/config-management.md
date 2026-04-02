---
name: config-management
tags: [config, settings, loading, merging, overrides]
skills: [json-api]
author: Arcee Coder Team
created: 2025-01-01
status: active
priority: high
---

## Description
Test the configuration system, including loading from multiple sources, merging strategies, validation, and override handling.

## Test Scenarios

### Scenario 1: Default Configuration Loading
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
  "response": "Configuration loaded from defaults",
  "metadata": {
    "config_source": "defaults",
    "values": {
      "model": "trinity-large-thinking",
      "api_key": "",
      "base_url": "https://api.arcee.ai",
      "permission_mode": "default",
      "permission_strictness": "medium",
      "max_turns": 200,
      "max_tokens": 16384,
      "budget_usd": null,
      "auto_model_routing": true,
      "intensity": "medium",
      "verbose": false
    },
    "validation_passed": true
  }
}
```
**Validation Criteria:**
- [ ] All default values loaded correctly
- [ ] No API key present by default
- [ ] Proper data types for each config value
- [ ] Config validation passes
- [ ] Missing config file handled gracefully

### Scenario 2: User-Level Config File Override
**Input:**
```json
{
  "prompt": "Load configuration from ~/.config/arcee-code/config.json",
  "context": {},
  "options": {
    "config_path": "/home/test/.config/arcee-code/config.json",
    "config_content": {
      "api_key": "test-api-key-123",
      "model": "trinity-medium-thinking",
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
**Expected Output:**
```json
{
  "response": "Configuration loaded from user config file",
  "metadata": {
    "config_source": "user_config",
    "values": {
      "api_key": "test-api-key-123",
      "model": "trinity-medium-thinking",
      "budget": 20.0,
      "timeout": 60,
      "strictness": "high",
      "intensity": "high",
      "stream": false,
      "debug": true,
      "color": false
    },
    "overrides_defaults": true,
    "validation_passed": true
  }
}
```
**Validation Criteria:**
- [ ] User config file found and parsed
- [ ] Values correctly override defaults
- [ ] Proper error handling for missing file
- [ ] Config validation passes
- [ ] Changes applied without restart

### Scenario 3: Project-Level Config Override
**Input:**
```json
{
  "prompt": "Load project configuration from .arcee/settings.json",
  "context": {
    "cwd": "/home/test/project",
    "project_config": {
      "permission_mode": "plan",
      "permission_strictness": "low",
      "hooks": {
        "pre_tool_use": [
          {"event": "write", "command": "echo 'Starting write...'"}
        ]
      }
    }
  },
  "options": {}
}
```
**Expected Output:**
```json
{
  "response": "Project configuration loaded and merged",
  "metadata": {
    "config_source": "project_config",
    "values": {
      "permission_mode": "plan",
      "permission_strictness": "low",
      "hooks_merged": true
    },
    "overrides_user_config": true,
    "validation_passed": true
  }
}
```
**Validation Criteria:**
- [ ] Project config file found and parsed
- [ ] Values correctly override user config
- [ ] Hooks merged correctly
- [ ] Permission modes updated
- [ ] Config validation passes

### Scenario 4: Environment Variable Overrides
**Input:**
```json
{
  "prompt": "Set environment variables and get configuration",
  "context": {
    "env": {
      "ARCEE_API_KEY": "env-api-key-456",
      "ARCEE_MODEL": "trinity-small-thinking",
      "ARCEE_PERMISSION_STRICTNESS": "high"
    }
  },
  "options": {}
}
```
**Expected Output:**
```json
{
  "response": "Environment variables loaded and applied",
  "metadata": {
    "config_source": "environment",
    "values": {
      "api_key": "env-api-key-456",
      "model": "trinity-small-thinking",
      "permission_strictness": "high"
    },
    "overrides_all": true,
    "validation_passed": true
  }
}
```
**Validation Criteria:**
- [ ] Environment variables correctly detected
- [ ] Values override other config sources
- [ ] Proper error handling for missing variables
- [ ] Security: Sensitive data masked
- [ ] Config validation passes

### Scenario 5: CLI Flag Overrides
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
  "response": "CLI flags processed and applied",
  "metadata": {
    "config_source": "cli_flags",
    "values": {
      "model": "trinity-medium-thinking",
      "budget": 5.0,
      "timeout": 15,
      "prompt_provided": true
    },
    "overrides_all": true,
    "validation_passed": true,
    "prompt_analyzed": true
  }
}
```
**Validation Criteria:**
- [ ] CLI flags parsed correctly
- [ ] Values override both defaults and user config
- [ ] Prompt processed with new settings
- [ ] Proper error messages for invalid flags
- [ ] Config validation passes

### Scenario 6: Config Merging Order
**Input:**
```json
{
  "prompt": "Test config merging with conflicting values",
  "context": {
    "user_config": {
      "model": "trinity-medium-thinking",
      "budget": 15.0
    },
    "project_config": {
      "model": "trinity-large-thinking",
      "timeout": 60
    },
    "env": {
      "ARCEE_MODEL": "trinity-small-thinking"
    },
    "cli_flags": {
      "model": "trinity-medium-thinking",
      "budget": 5.0
    }
  },
  "options": {}
}
```
**Expected Output:**
```json
{
  "response": "Configuration merging completed",
  "metadata": {
    "config_source": "merged",
    "final_values": {
      "model": "trinity-medium-thinking",
      "budget": 5.0,
      "timeout": 60
    },
    "merge_order": ["defaults", "user_config", "project_config", "environment", "cli"],
    "overrides_documentation": "CLI flags have highest priority",
    "validation_passed": true
  }
}
```
**Validation Criteria:**
- [ ] Correct merge order applied
- [ ] CLI flags have highest priority
- [ ] Environment variables override project config
- [ ] Project config overrides user config
- [ ] Defaults have lowest priority
- [ ] Config validation passes

### Scenario 7: Invalid Config File Handling
**Input:**
```json
{
  "prompt": "Load corrupted config file",
  "context": {
    "config_path": "/home/test/.config/arcee-code/config.json",
    "config_content": "{ invalid json }"
  },
  "options": {}
}
```
**Expected Output:**
```json
{
  "response": "Warning: Failed to parse config file. Using defaults.",
  "metadata": {
    "config_source": "defaults",
    "parse_error": true,
    "error_message": "expected value at line 1",
    "defaults_used": true,
    "validation_passed": true
  }
}
```
**Validation Criteria:**
- [ ] Parse error detected
- [ ] Warning message displayed
- [ ] Defaults used as fallback
- [ ] No crash
- [ ] Config validation passes

### Scenario 8: Config Validation Failures
**Input:**
```json
{
  "prompt": "Load config with invalid values",
  "context": {
    "config_path": "/home/test/.config/arcee-code/config.json",
    "config_content": {
      "max_turns": 0,
      "max_tokens": 0,
      "budget": -1.0
    }
  },
  "options": {}
}
```
**Expected Output:**
```json
{
  "response": "Configuration loaded with validation adjustments",
  "metadata": {
    "config_source": "user_config",
    "values_adjusted": true,
    "adjustments": {
      "max_turns": "Set to 1 (was 0)",
      "max_tokens": "Set to 16384 (was 0)",
      "budget": "Set to null (was -1.0)"
    },
    "validation_warnings": 3,
    "validation_passed": true
  }
}
```
**Validation Criteria:**
- [ ] Invalid values detected
- [ ] Values adjusted to safe defaults
- [ ] Validation warnings logged
- [ ] Config still usable
- [ ] No crash

### Edge Cases
- [ ] Missing config directory
- [ ] Config file with wrong permissions
- [ ] Config file with duplicate keys
- [ ] Config file with comments
- [ ] Config file with relative paths
- [ ] Config file with environment variables
- [ ] Config file with includes
- [ ] Config file with syntax errors
- [ ] Config file with type mismatches
- [ ] Config file with missing required fields

### Success Metrics
- Config loading success rate: >99%
- Merge order correctness: 100%
- Validation accuracy: 100%
- Performance impact: <100ms
- Error recovery: >95%

### Related Tests
- [cli-arguments](#)
- [api-integration](#)

### Changelog
- **2025-01-01**: Initial creation
- **2025-01-02**: Added validation testing
- **2025-01-03**: Added merge order scenarios