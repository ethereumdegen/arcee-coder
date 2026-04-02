# Arcee Code — Comprehensive Plan

## Overview

Arcee Code is a Rust-based AI coding assistant CLI, powered by Arcee AI. It targets a native Rust binary with full agentic coding functionality.

---

## Architecture Comparison

| Aspect | Arcee Code |
|--------|-------------------|
| Language | Rust |
| Runtime | Native binary |
| UI Framework | Ratatui + Crossterm |
| CLI Parser | Clap |
| HTTP Client | Reqwest |
| JSON Schema | Serde + schemars |
| Async Runtime | Tokio |
| Streaming | tokio Stream / SSE |
| Config | JSON files (serde_json) |
| Process Exec | tokio::process |

---

## Core Components

### 1. CLI Entry Point (`main.rs`)

**Responsibilities:**
- Parse CLI arguments via Clap
- Load configuration from `~/.arcee/config.json`
- Initialize API client
- Launch REPL or execute one-shot query
- Handle signals (Ctrl+C, Ctrl+D)

**CLI Interface:**
```
arcee [OPTIONS] [PROMPT]

Options:
  -m, --model <MODEL>          Override model
  -p, --permission-mode <MODE> Permission mode: default, auto, plan
  --resume                     Resume last session
  --verbose                    Verbose output
  --no-cache                   Disable prompt caching
  --max-turns <N>              Max agentic turns
  --budget <USD>               Max spend budget
  -h, --help                   Print help
  -V, --version                Print version
```

### 2. API Client (`api/`)

**Files:**
- `api/client.rs` — Anthropic Messages API client
- `api/types.rs` — Request/response types
- `api/streaming.rs` — SSE stream parser
- `api/retry.rs` — Exponential backoff + retry logic
- `api/errors.rs` — Error categorization

**Key Types:**
```rust
struct ApiClient {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
}

struct MessagesRequest {
    model: String,
    max_tokens: u32,
    system: Vec<SystemBlock>,
    messages: Vec<Message>,
    tools: Vec<ToolDefinition>,
    stream: bool,
}

enum StreamEvent {
    MessageStart { message: MessageResponse },
    ContentBlockStart { index: usize, content_block: ContentBlock },
    ContentBlockDelta { index: usize, delta: ContentDelta },
    ContentBlockStop { index: usize },
    MessageDelta { delta: MessageDelta, usage: Usage },
    MessageStop,
    Ping,
    Error { error: ApiError },
}
```

**Streaming Implementation:**
- Uses `reqwest` with `text/event-stream` accept header
- Parses SSE lines (`event:`, `data:`) into `StreamEvent` enum
- Buffers content deltas into complete `ContentBlock`s
- Yields accumulated `Message` when stream ends

**Retry Logic:**
- Categorize errors: rate_limit, server_error, auth_error, context_error
- Exponential backoff with jitter for rate_limit + server_error
- Respect `retry-after` header
- Max 3 retries for transient errors
- No retry for auth/context errors

### 3. Message Types (`messages/`)

**Files:**
- `messages/types.rs` — Core message types
- `messages/normalize.rs` — API normalization

**Key Types:**
```rust
enum Message {
    User(UserMessage),
    Assistant(AssistantMessage),
    System(SystemMessage),
}

struct UserMessage {
    content: Vec<ContentPart>,
}

struct AssistantMessage {
    content: Vec<ContentBlock>,
    stop_reason: StopReason,
    usage: Usage,
}

enum ContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: String, is_error: bool },
    Thinking { thinking: String },
}

enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    StopSequence,
}

struct Usage {
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
}
```

### 4. Tool System (`tools/`)

**Files:**
- `tools/mod.rs` — Tool trait + registry
- `tools/bash.rs` — Shell command execution
- `tools/read.rs` — File reading
- `tools/write.rs` — File writing
- `tools/edit.rs` — String replacement editing
- `tools/glob.rs` — File pattern matching
- `tools/grep.rs` — Content search (ripgrep)
- `tools/web_fetch.rs` — HTTP content fetching
- `tools/agent.rs` — Sub-agent spawning
- `tools/ask_user.rs` — User prompting

**Tool Trait:**
```rust
#[async_trait]
trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> serde_json::Value;
    fn is_read_only(&self, input: &serde_json::Value) -> bool;

    async fn call(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult>;

    fn check_permissions(
        &self,
        input: &serde_json::Value,
        context: &PermissionContext,
    ) -> PermissionResult;
}

struct ToolResult {
    content: String,
    is_error: bool,
}

struct ToolContext {
    cwd: PathBuf,
    permission_mode: PermissionMode,
}
```

**Tool Registry:**
```rust
struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    fn register(&mut self, tool: Box<dyn Tool>);
    fn get(&self, name: &str) -> Option<&dyn Tool>;
    fn all_definitions(&self) -> Vec<ToolDefinition>;
}
```

**Tool Implementations:**

| Tool | Input Schema | Behavior |
|------|-------------|----------|
| `Bash` | `{ command: String, timeout?: u64, description?: String }` | Execute shell command via `tokio::process::Command`, capture stdout+stderr, respect timeout |
| `Read` | `{ file_path: String, offset?: usize, limit?: usize }` | Read file with line numbers (`cat -n` style), truncate long lines at 2000 chars |
| `Write` | `{ file_path: String, content: String }` | Write file contents, create parent dirs if needed |
| `Edit` | `{ file_path: String, old_string: String, new_string: String, replace_all?: bool }` | Find and replace exact string in file, fail if not unique (unless replace_all) |
| `Glob` | `{ pattern: String, path?: String }` | Find files matching glob pattern using `globwalk` crate |
| `Grep` | `{ pattern: String, path?: String, glob?: String, output_mode?: String, context?: usize }` | Search file contents using `grep` crate with regex support |
| `WebFetch` | `{ url: String, prompt: String }` | Fetch URL, convert HTML to text, return content |
| `Agent` | `{ prompt: String, description: String }` | Spawn sub-conversation with isolated context |
| `AskUserQuestion` | `{ question: String }` | Prompt user for input |

### 5. Query Engine (`engine/`)

**Files:**
- `engine/mod.rs` — Main query loop
- `engine/context.rs` — System prompt + context assembly
- `engine/compact.rs` — Context compression
- `engine/cost.rs` — Cost tracking

**Query Loop (core algorithm):**
```rust
async fn query_loop(
    client: &ApiClient,
    messages: &mut Vec<Message>,
    system_prompt: &str,
    tools: &ToolRegistry,
    config: &Config,
) -> Result<()> {
    let mut turns = 0;
    loop {
        // 1. Build API request
        let request = build_request(messages, system_prompt, tools, config);

        // 2. Stream response
        let response = client.stream_messages(request).await?;

        // 3. Accumulate assistant message
        let assistant_msg = accumulate_stream(response).await?;
        messages.push(Message::Assistant(assistant_msg.clone()));

        // 4. Check stop reason
        if assistant_msg.stop_reason == StopReason::EndTurn {
            break;
        }

        // 5. Execute tool calls
        if assistant_msg.stop_reason == StopReason::ToolUse {
            let tool_results = execute_tools(&assistant_msg, tools, config).await?;
            messages.push(Message::User(UserMessage {
                content: tool_results,
            }));
        }

        turns += 1;
        if turns >= config.max_turns {
            break;
        }
    }
    Ok(())
}
```

**Tool Execution:**
```rust
async fn execute_tools(
    assistant_msg: &AssistantMessage,
    tools: &ToolRegistry,
    config: &Config,
) -> Result<Vec<ContentPart>> {
    let tool_uses: Vec<_> = assistant_msg.content.iter()
        .filter_map(|b| match b {
            ContentBlock::ToolUse { id, name, input } => Some((id, name, input)),
            _ => None,
        })
        .collect();

    let mut results = Vec::new();
    for (id, name, input) in tool_uses {
        // Permission check
        let tool = tools.get(name)?;
        let perm = check_permission(tool, input, config)?;

        let result = match perm {
            PermissionResult::Allow => tool.call(input.clone(), &context).await?,
            PermissionResult::Deny(reason) => ToolResult {
                content: format!("Permission denied: {}", reason),
                is_error: true
            },
            PermissionResult::Ask => {
                if prompt_user(tool, input)? {
                    tool.call(input.clone(), &context).await?
                } else {
                    ToolResult { content: "User denied".into(), is_error: true }
                }
            }
        };

        results.push(ContentPart::ToolResult {
            tool_use_id: id.clone(),
            content: result.content,
            is_error: result.is_error,
        });
    }
    Ok(results)
}
```

**System Prompt Assembly:**
```rust
fn build_system_prompt(config: &Config, cwd: &Path) -> String {
    let mut parts = vec![
        CORE_SYSTEM_PROMPT,           // Role + behavior instructions
        &tool_instructions(config),    // Tool usage guide
        &environment_context(cwd),     // OS, shell, cwd, git info
        &memory_context(cwd),          // ARCEE.md / memory files
    ];
    parts.join("\n\n")
}
```

### 6. Permission System (`permissions/`)

**Files:**
- `permissions/mod.rs` — Permission types + checking
- `permissions/rules.rs` — Rule matching
- `permissions/prompt.rs` — User prompting UI

**Types:**
```rust
enum PermissionMode {
    Default,    // Always ask
    Auto,       // Smart classifier
    Plan,       // Ask during plan, auto during execute
    Bypass,     // Allow all (dangerous)
}

enum PermissionResult {
    Allow,
    Deny(String),
    Ask,
}

struct PermissionRule {
    tool_name: String,
    pattern: Option<String>,  // e.g., "Bash(git *)"
    behavior: RuleBehavior,   // Allow, Deny, Ask
}
```

**Decision Flow:**
1. Check deny rules → if match, deny
2. Check allow rules → if match, allow
3. Check permission mode:
   - `Bypass` → allow
   - `Default` → ask user
   - `Auto` → classify then allow/ask
   - `Plan` → ask in plan phase, allow in execute phase

### 7. Configuration (`config/`)

**Files:**
- `config/mod.rs` — Config types + loading
- `config/paths.rs` — Config file paths

**Config Structure:**
```rust
struct Config {
    // API
    api_key: String,
    base_url: String,
    model: String,

    // Behavior
    permission_mode: PermissionMode,
    max_turns: usize,
    max_tokens: u32,
    budget_usd: Option<f64>,

    // Permission rules
    allow_rules: Vec<PermissionRule>,
    deny_rules: Vec<PermissionRule>,

    // Paths
    config_dir: PathBuf,  // ~/.arcee/
    cwd: PathBuf,
}
```

**Config Loading Order:**
1. Defaults
2. `~/.arcee/config.json` (user-level)
3. `.arcee/settings.json` (project-level)
4. Environment variables (`ANTHROPIC_API_KEY`, `ANTHROPIC_BASE_URL`, etc.)
5. CLI arguments (highest priority)

### 8. Terminal UI (`ui/`)

**Files:**
- `ui/mod.rs` — REPL main loop
- `ui/render.rs` — Message rendering
- `ui/input.rs` — Input handling (line editor)
- `ui/markdown.rs` — Markdown rendering for terminal
- `ui/spinner.rs` — Progress indicators
- `ui/theme.rs` — Colors and styling

**REPL Architecture:**
```rust
async fn repl(config: Config) -> Result<()> {
    let mut terminal = setup_terminal()?;
    let client = ApiClient::new(&config);
    let tools = build_tool_registry(&config);
    let mut messages: Vec<Message> = Vec::new();
    let mut session = Session::new();

    loop {
        // 1. Render current state
        render(&mut terminal, &messages, &session)?;

        // 2. Get user input
        let input = read_input(&mut terminal).await?;

        match input {
            Input::Text(text) => {
                // Handle slash commands
                if text.starts_with('/') {
                    handle_command(&text, &mut session, &config)?;
                    continue;
                }

                // Add user message
                messages.push(Message::user(text));

                // 3. Run query loop (streaming)
                query_loop_streaming(
                    &client,
                    &mut messages,
                    &tools,
                    &config,
                    &mut terminal,
                ).await?;
            }
            Input::Interrupt => break,
            Input::Eof => break,
        }
    }

    restore_terminal(terminal)?;
    Ok(())
}
```

**Rendering:**
- Assistant text → rendered as markdown (bold, italic, code blocks with syntax highlighting)
- Tool use → show tool name + input summary
- Tool result → show output (truncated if long)
- Thinking → dimmed/italic text
- Streaming → character-by-character append with cursor

**Input Handling:**
- Line editor with history (rustyline or custom)
- Multi-line input (shift+enter or paste detection)
- Ctrl+C → interrupt current generation
- Ctrl+D → exit
- Up/Down → history navigation

### 9. Session Management (`session/`)

**Files:**
- `session/mod.rs` — Session state
- `session/storage.rs` — Session persistence

**Session Storage:**
```rust
struct Session {
    id: String,
    messages: Vec<Message>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    cwd: PathBuf,
    model: String,
    total_cost: f64,
    total_input_tokens: u64,
    total_output_tokens: u64,
}

// Sessions stored at: ~/.arcee/sessions/{session_id}.json
```

### 10. Slash Commands (`commands/`)

**Built-in Commands:**

| Command | Behavior |
|---------|----------|
| `/help` | Show help text |
| `/clear` | Clear conversation |
| `/compact` | Compress context (summarize old messages) |
| `/config` | Show/edit config |
| `/model <name>` | Switch model |
| `/quit` or `/exit` | Exit REPL |
| `/cost` | Show token usage and cost |
| `/history` | Show conversation history |

---

## Directory Structure

```
arcee-code/
├── Cargo.toml
├── PLAN.md
├── src/
│   ├── main.rs              # Entry point, CLI parsing
│   ├── lib.rs               # Library root
│   ├── api/
│   │   ├── mod.rs            # API module
│   │   ├── client.rs         # Anthropic API client
│   │   ├── types.rs          # API request/response types
│   │   ├── streaming.rs      # SSE stream parser
│   │   ├── retry.rs          # Retry logic
│   │   └── errors.rs         # Error types
│   ├── messages/
│   │   ├── mod.rs            # Message types
│   │   ├── types.rs          # Core types
│   │   └── normalize.rs      # API normalization
│   ├── tools/
│   │   ├── mod.rs            # Tool trait + registry
│   │   ├── bash.rs           # Shell execution
│   │   ├── read.rs           # File read
│   │   ├── write.rs          # File write
│   │   ├── edit.rs           # File edit
│   │   ├── glob.rs           # File glob
│   │   ├── grep.rs           # Content search
│   │   ├── web_fetch.rs      # URL fetching
│   │   ├── agent.rs          # Sub-agent
│   │   └── ask_user.rs       # User prompt
│   ├── engine/
│   │   ├── mod.rs            # Query engine
│   │   ├── context.rs        # System prompt builder
│   │   ├── compact.rs        # Context compression
│   │   └── cost.rs           # Cost tracking
│   ├── permissions/
│   │   ├── mod.rs            # Permission system
│   │   ├── rules.rs          # Rule matching
│   │   └── prompt.rs         # Permission prompts
│   ├── config/
│   │   ├── mod.rs            # Configuration
│   │   └── paths.rs          # Path resolution
│   ├── session/
│   │   ├── mod.rs            # Session management
│   │   └── storage.rs        # Persistence
│   ├── ui/
│   │   ├── mod.rs            # REPL main loop
│   │   ├── render.rs         # Message rendering
│   │   ├── input.rs          # Input handling
│   │   ├── markdown.rs       # Terminal markdown
│   │   ├── spinner.rs        # Progress indicators
│   │   └── theme.rs          # Colors/styling
│   └── commands/
│       └── mod.rs            # Slash commands
├── tests/
│   ├── api_test.rs
│   ├── tools_test.rs
│   └── engine_test.rs
└── .arcee/                   # Default config template
    └── config.json
```

---

## Dependencies (Cargo.toml)

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["stream", "json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
crossterm = "0.28"
ratatui = "0.29"
rustyline = "14"
glob = "0.3"
regex = "1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
dirs = "6"
anyhow = "1"
thiserror = "2"
async-trait = "0.1"
futures = "0.3"
tokio-stream = "0.1"
syntect = "5"             # Syntax highlighting for code blocks
termimad = "0.30"         # Terminal markdown rendering
indicatif = "0.17"        # Progress bars/spinners
colored = "3"             # Terminal colors
walkdir = "2"             # Directory walking
ignore = "0.4"            # .gitignore-aware walking (for grep)
html2text = "0.12"        # HTML to text (for web_fetch)
```

---

## Build Phases

### Phase 1: Foundation (Core Types + API Client)
1. `Cargo.toml` + project structure
2. API types (`api/types.rs`)
3. API client with streaming (`api/client.rs`, `api/streaming.rs`)
4. Message types (`messages/types.rs`)
5. Config loading (`config/`)
6. Basic CLI entry point

### Phase 2: Tool System
1. Tool trait definition (`tools/mod.rs`)
2. Bash tool (`tools/bash.rs`)
3. Read tool (`tools/read.rs`)
4. Write tool (`tools/write.rs`)
5. Edit tool (`tools/edit.rs`)
6. Glob tool (`tools/glob.rs`)
7. Grep tool (`tools/grep.rs`)
8. Tool registry

### Phase 3: Query Engine
1. System prompt assembly (`engine/context.rs`)
2. Query loop with tool execution (`engine/mod.rs`)
3. Permission checking (`permissions/`)
4. Cost tracking (`engine/cost.rs`)
5. Error handling + retries (`api/retry.rs`)

### Phase 4: Terminal UI
1. Terminal setup + restoration (`ui/mod.rs`)
2. Input handling with history (`ui/input.rs`)
3. Streaming message rendering (`ui/render.rs`)
4. Markdown rendering (`ui/markdown.rs`)
5. Progress spinners (`ui/spinner.rs`)
6. Slash commands (`commands/`)

### Phase 5: Session + Polish
1. Session save/resume (`session/`)
2. Context compaction (`engine/compact.rs`)
3. WebFetch tool (`tools/web_fetch.rs`)
4. Agent tool (`tools/agent.rs`)
5. AskUser tool (`tools/ask_user.rs`)

---

## Cost Model Reference

| Model | Input ($/1M) | Output ($/1M) | Cache Write ($/1M) | Cache Read ($/1M) |
|-------|-------------|---------------|--------------------|--------------------|
| arcee-sonnet | $3.00 | $15.00 | $3.75 | $0.30 |
| arcee-opus | $15.00 | $75.00 | $18.75 | $1.50 |
| arcee-haiku | $0.80 | $4.00 | $1.00 | $0.08 |

---

## Key Design Decisions

1. **Ratatui over TUI alternatives** — Most mature Rust terminal UI, good crossterm integration
2. **Reqwest over hyper** — Higher-level HTTP with streaming support, less boilerplate
3. **Clap derive over manual** — Type-safe CLI parsing with minimal code
4. **anyhow + thiserror** — anyhow for application errors, thiserror for library error types
5. **Tool trait objects** — `Box<dyn Tool>` for runtime polymorphism in tool registry
6. **Tokio** — Industry standard async runtime, required by reqwest
7. **SSE manual parsing** — No established Rust SSE crate matches our needs; simple line parser
8. **JSON config files** — Standard JSON configuration format
