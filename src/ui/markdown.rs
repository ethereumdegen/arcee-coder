use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

/// Streaming-aware markdown renderer with syntax highlighting for code blocks.
///
/// Tracks state across `push_text()` calls to handle code blocks that span
/// multiple streaming chunks. Non-code text gets basic ANSI formatting.
pub struct MarkdownRenderer {
    ss: SyntaxSet,
    ts: ThemeSet,
    theme_name: String,
    /// Are we currently inside a fenced code block?
    in_code_block: bool,
    /// Language tag from the opening fence (e.g. "rust", "python").
    code_language: String,
    /// Accumulated lines inside the current code block.
    code_buffer: String,
    /// Partial line buffer for incomplete lines during streaming.
    line_buffer: String,
    /// Whether to apply formatting (disabled in non-terminal contexts).
    enabled: bool,
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        Self {
            ss,
            ts,
            theme_name: "base16-ocean.dark".to_string(),
            in_code_block: false,
            code_language: String::new(),
            code_buffer: String::new(),
            line_buffer: String::new(),
            enabled: true,
        }
    }

    /// Process incoming streamed text and return ANSI-formatted output.
    /// Call this with each chunk from the streaming API.
    pub fn push_text(&mut self, text: &str) -> String {
        if !self.enabled {
            return text.to_string();
        }

        let mut output = String::new();
        self.line_buffer.push_str(text);

        // Process complete lines, keep incomplete last line in buffer
        while let Some(newline_pos) = self.line_buffer.find('\n') {
            let line = self.line_buffer[..=newline_pos].to_string();
            self.line_buffer = self.line_buffer[newline_pos + 1..].to_string();

            let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');

            if !self.in_code_block && trimmed.starts_with("```") {
                // Opening fence
                self.in_code_block = true;
                self.code_language = trimmed.trim_start_matches('`').trim().to_string();
                self.code_buffer.clear();
                // Print the fence with dim styling
                output.push_str(&format!("\x1b[2m{line}\x1b[0m"));
            } else if self.in_code_block && trimmed == "```" {
                // Closing fence — highlight accumulated code
                output.push_str(&self.highlight_code_block());
                output.push_str(&format!("\x1b[2m{line}\x1b[0m"));
                self.in_code_block = false;
                self.code_language.clear();
                self.code_buffer.clear();
            } else if self.in_code_block {
                // Inside code block — accumulate
                self.code_buffer.push_str(&line);
            } else {
                // Regular markdown line
                output.push_str(&format_markdown_line(trimmed));
                output.push('\n');
            }
        }

        // If we have a partial line and we're NOT in a code block,
        // flush it immediately (streaming text appearance)
        if !self.line_buffer.is_empty() && !self.in_code_block {
            // For inline streaming, just return the partial content with inline formatting
            let partial = std::mem::take(&mut self.line_buffer);
            output.push_str(&format_inline(&partial));
        }

        output
    }

    /// Flush any remaining buffered content (call at end of stream).
    pub fn flush(&mut self) -> String {
        let mut output = String::new();

        if self.in_code_block && !self.code_buffer.is_empty() {
            // Unterminated code block — highlight what we have
            output.push_str(&self.highlight_code_block());
            self.in_code_block = false;
        }

        if !self.line_buffer.is_empty() {
            let remaining = std::mem::take(&mut self.line_buffer);
            output.push_str(&format_inline(&remaining));
        }

        output
    }

    /// Highlight the accumulated code buffer using syntect.
    fn highlight_code_block(&self) -> String {
        let syntax = if !self.code_language.is_empty() {
            self.ss
                .find_syntax_by_token(&self.code_language)
                .unwrap_or_else(|| self.ss.find_syntax_plain_text())
        } else {
            self.ss.find_syntax_plain_text()
        };

        let theme = match self.ts.themes.get(&self.theme_name) {
            Some(t) => t,
            None => return self.code_buffer.clone(),
        };

        let mut h = HighlightLines::new(syntax, theme);
        let mut result = String::new();

        for line in LinesWithEndings::from(&self.code_buffer) {
            match h.highlight_line(line, &self.ss) {
                Ok(ranges) => {
                    let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
                    result.push_str(&escaped);
                }
                Err(_) => {
                    result.push_str(line);
                }
            }
        }
        result.push_str("\x1b[0m");
        result
    }

    /// Disable formatting (e.g., when output is piped).
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

/// Apply basic markdown formatting to a single complete line.
fn format_markdown_line(line: &str) -> String {
    // Headers
    if line.starts_with("### ") {
        return format!("\x1b[1;36m{}\x1b[0m", &line[4..]);
    }
    if line.starts_with("## ") {
        return format!("\x1b[1;36m{}\x1b[0m", &line[3..]);
    }
    if line.starts_with("# ") {
        return format!("\x1b[1;35m{}\x1b[0m", &line[2..]);
    }

    // Horizontal rules
    if line == "---" || line == "***" || line == "___" {
        return "\x1b[2m────────────────────────────────\x1b[0m".to_string();
    }

    // Bullet points — highlight the bullet
    if let Some(rest) = line.strip_prefix("- ") {
        return format!("\x1b[33m•\x1b[0m {}", format_inline(rest));
    }
    if let Some(rest) = line.strip_prefix("* ") {
        return format!("\x1b[33m•\x1b[0m {}", format_inline(rest));
    }

    // Numbered lists
    if let Some(dot_pos) = line.find(". ") {
        let prefix = &line[..dot_pos];
        if prefix.chars().all(|c| c.is_ascii_digit()) {
            return format!(
                "\x1b[33m{}.\x1b[0m {}",
                prefix,
                format_inline(&line[dot_pos + 2..])
            );
        }
    }

    // Blockquotes
    if let Some(rest) = line.strip_prefix("> ") {
        return format!("\x1b[2;3m│ {}\x1b[0m", format_inline(rest));
    }

    format_inline(line)
}

/// Apply inline markdown formatting: **bold**, *italic*, `code`.
fn format_inline(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + 32);
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Bold: **text**
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end) = find_closing(&chars, i + 2, &['*', '*']) {
                let inner: String = chars[i + 2..end].iter().collect();
                result.push_str(&format!("\x1b[1m{inner}\x1b[22m"));
                i = end + 2;
                continue;
            }
        }

        // Italic: *text*
        if chars[i] == '*' && (i + 1 < len && chars[i + 1] != '*') {
            if let Some(end) = find_single_closing(&chars, i + 1, '*') {
                let inner: String = chars[i + 1..end].iter().collect();
                result.push_str(&format!("\x1b[3m{inner}\x1b[23m"));
                i = end + 1;
                continue;
            }
        }

        // Inline code: `text`
        if chars[i] == '`' && (i + 1 < len && chars[i + 1] != '`') {
            if let Some(end) = find_single_closing(&chars, i + 1, '`') {
                let inner: String = chars[i + 1..end].iter().collect();
                result.push_str(&format!("\x1b[36m{inner}\x1b[0m"));
                i = end + 1;
                continue;
            }
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Find closing double-char marker (e.g., **).
fn find_closing(chars: &[char], start: usize, marker: &[char; 2]) -> Option<usize> {
    let len = chars.len();
    for j in start..len.saturating_sub(1) {
        if chars[j] == marker[0] && chars[j + 1] == marker[1] {
            return Some(j);
        }
    }
    None
}

/// Find closing single-char marker (e.g., * or `).
fn find_single_closing(chars: &[char], start: usize, marker: char) -> Option<usize> {
    for j in start..chars.len() {
        if chars[j] == marker {
            return Some(j);
        }
    }
    None
}
