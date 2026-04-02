use std::path::Path;

/// Build the system prompt for the conversation.
pub fn build_system_prompt(cwd: &Path, model: &str) -> String {
    let env_context = build_environment_context(cwd);
    let tool_instructions = build_tool_instructions();
    let memory = load_memory_files(cwd);

    let mut prompt = String::new();

    prompt.push_str(CORE_PROMPT);
    prompt.push_str("\n\n");
    prompt.push_str(&tool_instructions);
    prompt.push_str("\n\n# Environment\n");
    prompt.push_str(&env_context);

    if !memory.is_empty() {
        prompt.push_str("\n\n# Project Memory\n");
        prompt.push_str(&memory);
    }

    prompt.push_str(&format!("\n\nYou are powered by {model}.\n"));

    prompt
}

const CORE_PROMPT: &str = r#"You are Arcee Code, an AI coding assistant powered by Arcee AI, running in the terminal.
You help users with software engineering tasks including writing code, debugging,
refactoring, explaining code, running commands, and managing files.

# Guidelines
- Be concise and direct. Lead with the answer or action.
- Read files before modifying them. Understand existing code before suggesting changes.
- Prefer editing existing files over creating new ones.
- Use the appropriate tool for each task (Read for files, Bash for commands, etc.).
- Write safe, secure code. Avoid introducing vulnerabilities.
- Don't over-engineer. Make only the changes that are needed.
- When executing commands, use absolute paths to avoid directory confusion.
- For file edits, preserve exact indentation and formatting.
- Ask the user when you need clarification rather than guessing."#;

fn build_tool_instructions() -> String {
    r#"# Tool Usage
- Use Read to read files (not cat/head/tail via Bash)
- Use Write to create new files (not echo/cat via Bash)
- Use Edit for targeted string replacements in existing files (not sed/awk)
- Use Glob to find files by pattern (not find/ls via Bash)
- Use Grep to search file contents (not grep/rg via Bash)
- Use Bash for system commands, git operations, running tests, installing packages
- Use WebFetch to retrieve web content
- Use AskUserQuestion when you need user input"#
        .to_string()
}

fn build_environment_context(cwd: &Path) -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());

    let git_info = get_git_info(cwd);

    let mut ctx = format!(
        "- Working directory: {}\n- Platform: {os}/{arch}\n- Shell: {shell}",
        cwd.display()
    );

    if let Some(info) = git_info {
        ctx.push_str(&format!("\n- Git: {info}"));
    }

    ctx
}

fn get_git_info(cwd: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(cwd)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let branch = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(cwd)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "detached".to_string());

    Some(format!("branch={branch}"))
}

fn load_memory_files(cwd: &Path) -> String {
    let mut memory = String::new();

    // Check for ARCEE.md project memory file
    let arcee_md = cwd.join("ARCEE.md");
    if arcee_md.exists() {
        if let Ok(content) = std::fs::read_to_string(&arcee_md) {
            memory.push_str("## ARCEE.md\n");
            memory.push_str(&content);
            memory.push('\n');
        }
    }

    // Check for .arcee/memory/
    let arcee_memory = cwd.join(".arcee").join("memory");
    if arcee_memory.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&arcee_memory) {
            for entry in entries.flatten() {
                if entry.path().extension().is_some_and(|ext| ext == "md") {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        memory.push_str(&format!(
                            "\n## {}\n{}\n",
                            entry.file_name().to_string_lossy(),
                            content
                        ));
                    }
                }
            }
        }
    }

    memory
}
