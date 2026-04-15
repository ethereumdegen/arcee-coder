//! Structured tool output following the AXI principles:
//!
//! - **Pre-computed aggregates** via `summary`
//! - **Structured body** using TOON or plain text
//! - **Definitive empty states** via [`ToolBody::Empty`]
//! - **Smart truncation** with a `[truncated: ...]` footer and an explicit
//!   `how_to_see_more` hint, plus a `full=true` escape hatch honored per-tool
//! - **Contextual disclosure** via a list of `next_steps`
//!
//! A tool returns a `ToolOutput` and the engine calls [`ToolOutput::render`]
//! to produce the canonical string sent back to the model.

use crate::toon::ToonValue;

/// Canonical output of a tool execution.
#[derive(Debug, Clone)]
pub struct ToolOutput {
    /// One-line aggregate summary, e.g. `"42 matches in 7 files"`.
    pub summary: Option<String>,
    /// Main payload.
    pub body: ToolBody,
    /// Truncation metadata. When `None`, the output is complete.
    pub truncation: Option<Truncation>,
    /// Contextual disclosure hints the model can follow up with.
    pub next_steps: Vec<String>,
    /// Whether this result should be reported as an error to the model.
    pub is_error: bool,
}

/// Payload body of a [`ToolOutput`].
#[derive(Debug, Clone)]
pub enum ToolBody {
    /// Free-form text. Used by Bash stdout, Read content, Edit diffs, etc.
    Text(String),
    /// Structured TOON table or map.
    Toon(ToonValue),
    /// Definitive empty state with a human explanation of why no data came back.
    Empty(String),
}

/// Truncation metadata. Rendered as a `[truncated: ...]` footer.
#[derive(Debug, Clone)]
pub struct Truncation {
    pub shown: usize,
    pub total: usize,
    /// Unit: `"lines"`, `"matches"`, `"bytes"`, `"files"`, etc.
    pub unit: &'static str,
    /// Human-readable hint telling the model how to see more.
    pub how_to_see_more: String,
}

impl Default for ToolOutput {
    fn default() -> Self {
        Self {
            summary: None,
            body: ToolBody::Text(String::new()),
            truncation: None,
            next_steps: Vec::new(),
            is_error: false,
        }
    }
}

impl ToolOutput {
    /// Blank successful output. Fill in with builder methods.
    pub fn success() -> Self {
        Self::default()
    }

    /// Error output carrying an explanation.
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            body: ToolBody::Text(msg.into()),
            is_error: true,
            ..Self::default()
        }
    }

    /// Definitive empty result.
    pub fn empty(reason: impl Into<String>) -> Self {
        Self {
            body: ToolBody::Empty(reason.into()),
            ..Self::default()
        }
    }

    pub fn with_summary(mut self, s: impl Into<String>) -> Self {
        self.summary = Some(s.into());
        self
    }

    pub fn with_body(mut self, b: ToolBody) -> Self {
        self.body = b;
        self
    }

    pub fn with_text(mut self, t: impl Into<String>) -> Self {
        self.body = ToolBody::Text(t.into());
        self
    }

    pub fn with_toon(mut self, v: ToonValue) -> Self {
        self.body = ToolBody::Toon(v);
        self
    }

    pub fn with_truncation(mut self, t: Truncation) -> Self {
        self.truncation = Some(t);
        self
    }

    pub fn with_next_step(mut self, s: impl Into<String>) -> Self {
        self.next_steps.push(s.into());
        self
    }

    /// Replace the next-steps vector wholesale.
    pub fn with_next_steps<I, S>(mut self, steps: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.next_steps = steps.into_iter().map(Into::into).collect();
        self
    }

    /// If `full` is true, drop any pending truncation metadata so the full
    /// body is emitted without a truncation footer. This is the AXI `--full`
    /// escape hatch, applied after tools decide their own truncation policy.
    pub fn drop_truncation_if_full(mut self, full: bool) -> Self {
        if full {
            self.truncation = None;
        }
        self
    }

    /// Render the canonical string representation the model sees.
    ///
    /// Layout (all sections optional):
    ///
    /// ```text
    /// <summary>
    ///
    /// <body>
    ///
    /// [truncated: shown S/T unit. how_to_see_more]
    ///
    /// Next:
    ///  - <step 1>
    ///  - <step 2>
    /// ```
    pub fn render(&self) -> String {
        let mut sections: Vec<String> = Vec::new();

        if let Some(s) = &self.summary {
            if !s.is_empty() {
                sections.push(s.clone());
            }
        }

        let body_text = match &self.body {
            ToolBody::Text(t) => t.clone(),
            ToolBody::Toon(v) => v.render(),
            ToolBody::Empty(reason) => format!("0 results. {}", reason.trim()),
        };
        if !body_text.is_empty() {
            sections.push(body_text);
        }

        if let Some(t) = &self.truncation {
            sections.push(format!(
                "[truncated: shown {}/{} {}. {}]",
                t.shown, t.total, t.unit, t.how_to_see_more
            ));
        }

        if !self.next_steps.is_empty() {
            let mut block = String::from("Next:");
            for step in &self.next_steps {
                block.push_str("\n - ");
                block.push_str(step);
            }
            sections.push(block);
        }

        sections.join("\n\n")
    }

    /// Produce a short preview suitable for TUI display (first line, capped).
    pub fn preview(&self) -> String {
        if let Some(s) = &self.summary {
            return s.clone();
        }
        let text = match &self.body {
            ToolBody::Text(t) => t.clone(),
            ToolBody::Toon(v) => v.render(),
            ToolBody::Empty(reason) => format!("0 results. {}", reason),
        };
        text.lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(500)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_with_text_renders() {
        let o = ToolOutput::success().with_text("hello world");
        assert_eq!(o.render(), "hello world");
    }

    #[test]
    fn summary_body_truncation_next_steps_all_rendered() {
        let o = ToolOutput::success()
            .with_summary("42 matches in 7 files")
            .with_text("match 1\nmatch 2")
            .with_truncation(Truncation {
                shown: 2,
                total: 42,
                unit: "matches",
                how_to_see_more: "raise head_limit".into(),
            })
            .with_next_step("Use full=true for complete output");

        let rendered = o.render();
        assert!(rendered.starts_with("42 matches in 7 files"));
        assert!(rendered.contains("match 1\nmatch 2"));
        assert!(rendered.contains("[truncated: shown 2/42 matches. raise head_limit]"));
        assert!(rendered.contains("Next:\n - Use full=true for complete output"));
    }

    #[test]
    fn full_flag_drops_truncation() {
        let o = ToolOutput::success()
            .with_text("body")
            .with_truncation(Truncation {
                shown: 1,
                total: 100,
                unit: "lines",
                how_to_see_more: "pass offset".into(),
            })
            .drop_truncation_if_full(true);
        let rendered = o.render();
        assert!(!rendered.contains("[truncated"));
    }

    #[test]
    fn empty_body_renders_definitively() {
        let o = ToolOutput::empty("Pattern did not match any files");
        assert_eq!(o.render(), "0 results. Pattern did not match any files");
    }

    #[test]
    fn error_output_marked_is_error() {
        let o = ToolOutput::error("boom");
        assert!(o.is_error);
        assert!(o.render().contains("boom"));
    }
}
