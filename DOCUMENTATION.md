# Arcee Coder Documentation

## Overview

Arcee Coder is an AI-powered coding assistant CLI built with Rust. It leverages Arcee AI to provide intelligent coding assistance, including code generation, debugging, refactoring, and explanation capabilities.

## Architecture

### Core Components

#### 1. Main Entry Point (`src/main.rs`)
- CLI interface using Clap for command parsing
- Handles configuration loading and API key management
- Supports interactive REPL, one-shot mode, and stdin piping

#### 2. Configuration System (`src/config/`)
- **mod.rs**: Main configuration module
- **paths.rs**: Path management for config files and directories

#### 3. Permissions System (`src/permissions/`)
- **mod.rs**: Permission mode handling (default, auto, plan, bypass)
- **PermissionStrictness**: High, medium, low strictness levels

#### 4. Session Management (`src/session/`)
- **mod.rs**: Session creation, loading, and management
- **storage.rs**: Persistent storage for conversation history

#### 5. API Client (`src/api/`)
- **mod.rs**: API module
- **client.rs**: HTTP client implementation
- **errors.rs**: Error handling and types
- **streaming.rs**: Streaming response support
- **retry.rs**: Retry logic with exponential backoff
- **types.rs**: API request/response types

#### 6. UI System (`src/ui/`)
- **mod.rs**: Main UI module
- **render.rs**: Terminal rendering and output
- **thinking.rs**: AI thinking visualization
- **input_queue.rs**: Input handling and queuing

#### 7. Tools (`src/tools/`)
- **mod.rs**: Tool system
- **agent.rs**: Agent execution
- **bash.rs**: Bash command execution
- **edit.rs**: File editing capabilities
- **glob.rs**: File pattern matching
- **grep.rs**: Content searching
- **path_safety.rs**: Path validation
- **read.rs**: File reading
- **write.rs**: File writing
- **task_create.rs**: Task creation
- **task_get.rs**: Task retrieval
- **task_list.rs**: Task listing
- **task_store.rs**: Task storage
- **task_update.rs**: Task updating
- **worktree.rs**: Git worktree management
- **plan_mode.rs**: Planning mode
- **notebook_edit.rs**: Jupyter notebook editing
- **ask_user.rs**: User interaction
- **skill.rs**: Skill execution
- **web_fetch.rs**: Web content fetching
- **web_search.rs**: Web searching

#### 8. Message System (`src/messages/`)
- **mod.rs**: Message handling
- **types.rs**: Message types and structures
- **normalize.rs**: Message normalization

#### 9. Engine (`src/engine/`)
- **mod.rs**: Main engine module
- **context.rs**: Context management
- **compact.rs**: Code compaction
- **cost.rs**: Cost calculation
- **model_router.rs**: Model routing and selection

## Installation

### Prerequisites
- Rust toolchain (stable)
- Cargo

### Steps
```bash
# Clone the repository
git clone https://github.com/andy-ai/arcee-coder.git

# Navigate to the project directory
cd arcee-coder

# Build and install the CLI
cargo install --path .

# Alternatively, run directly
cargo run
```

## Usage

### Basic Commands
```bash
# Start the assistant in interactive mode
arcee-coder

# One-shot code generation
arcee-coder "Write a Rust function that calculates Fibonacci numbers"

# Specify a model
arcee-coder --model "trinity-large-thinking" "Explain this code"

# Resume previous session
arcee-coder --resume

# Set budget limit
arcee-coder --budget 5.0 "Write a complex algorithm"
```

### Interactive Mode
When you run `arcee-coder` without arguments, you enter an interactive REPL:
1. The assistant displays a thinking visualization
2. You can type prompts or commands
3. The assistant generates responses with syntax highlighting
4. Conversation history is saved in sessions

### Configuration
Arcee Coder loads configuration from `~/.arcee/config.json`. You can set:
- `api_key`: Your Arcee AI API key
- Other settings are managed via CLI flags

### Permissions
The tool supports different permission modes:
- **default**: Standard permissions
- **auto**: Automatic permission handling
- **plan**: Planning mode only
- **bypass**: Bypass permissions (use with caution)

## Testing with moderac

The `moderac` CLI is a separate tool for prompt-based testing of AI agents. It's used to test the Arcee Coder project through a series of markdown-based tests.

### Available Tests

The project includes 12 moderac tests covering various aspects:

1. **api-integration.md** - Tests API client integration with AI service
2. **cli-version.md** - Tests CLI version handling
3. **command-parsing.md** - Tests command parsing logic
4. **config-management.md** - Tests configuration management
5. **duplicate-email.md** - Tests duplicate email handling
6. **file-operations.md** - Tests file operations
7. **slash-commands.md** - Tests slash command handling
8. **slash-commit.md** - Tests commit command
9. **slash-review-pr.md** - Tests PR review command
10. **slash-simplify.md** - Tests code simplification
11. **thinking-mode.md** - Tests thinking mode functionality
12. **ui-interaction.md** - Tests UI interaction
13. **user-signup.md** - Tests user signup flow

### Running Tests
```bash
# List all tests
moderac list

# Run all tests
moderac test

# Run a specific test
moderac test api-integration.md

# Run with JSON output
moderac test --json
```

### Test Format
Each test is a markdown file with:
- **Metadata**: name, tags, skills, expected outcome
- **Test description**: What the test should verify
- **Requirements**: Specific behaviors to test

## Development

### Project Structure
```
arcee-coder/
├── src/              # Main source code
│   ├── api/          # API client
│   ├── config/       # Configuration
│   ├── engine/       # Core engine
│   ├── messages/     # Message handling
│   ├── permissions/  # Permission system
│   ├── session/      # Session management
│   ├── tools/        # Various tools
│   ├── ui/           # User interface
│   └── lib.rs        # Library entry point
├── moderac-tests/    # Prompt-based tests
├── Cargo.toml        # Cargo configuration
├── README.md         # Project readme
└── DOCUMENTATION.md  # This file
```

### Building
```bash
# Build the project
cargo build

# Build for release
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Clippy linting
cargo clippy
```

### Contributing
1. Fork the repository
2. Create a feature branch
3. Implement your changes
4. Add tests for new functionality
5. Submit a pull request

## License

MIT License

## Support

For issues or feature requests, please open an issue on GitHub.

## Acknowledgements

- Arcee AI for powering the intelligent assistance
- The Rust community for creating such a robust ecosystem
- All contributors to the open-source tools used in this project