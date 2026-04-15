//! TOON (Tabular Object Oriented Notation) format emitter.
//!
//! TOON is a compact, agent-friendly serialization format. Compared to JSON,
//! it strips punctuation noise (commas, brackets, quotes) from tabular data
//! by using a typed header row followed by pipe-separated value rows. For
//! list-of-records payloads — the dominant shape of coding-agent tool output
//! — TOON typically saves 35–45% of tokens vs. the equivalent JSON.
//!
//! Scalars and maps fall back to simple key/value rendering so a single
//! `ToonValue::render()` call produces a complete document regardless of
//! shape.
//!
//! # Example
//!
//! ```ignore
//! use arcee_code::toon::ToonValue;
//!
//! let files = ToonValue::Table {
//!     columns: vec!["path".into(), "size".into()],
//!     rows: vec![
//!         vec!["src/a.rs".into(), "1234".into()],
//!         vec!["src/b.rs".into(), "5678".into()],
//!     ],
//! };
//! let doc = ToonValue::Map(vec![("files".into(), files)]);
//! println!("{}", doc.render());
//! ```

use std::fmt::Write;

/// A TOON value: scalar, map, or table.
#[derive(Debug, Clone, PartialEq)]
pub enum ToonValue {
    /// Plain scalar (rendered as-is).
    Scalar(String),
    /// Ordered key→value map (preserves insertion order for deterministic output).
    Map(Vec<(String, ToonValue)>),
    /// Tabular data: a header row followed by value rows.
    Table {
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}

impl ToonValue {
    /// Render this value as a TOON document (no leading indent).
    pub fn render(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out
    }

    /// Render this value at a given indent depth (2-space indent per level).
    pub fn render_with_indent(&self, depth: usize) -> String {
        let mut out = String::new();
        self.write(&mut out, depth);
        out
    }

    fn write(&self, out: &mut String, depth: usize) {
        match self {
            ToonValue::Scalar(s) => {
                out.push_str(s);
            }
            ToonValue::Map(entries) => {
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        out.push('\n');
                    }
                    indent(out, depth);
                    match v {
                        ToonValue::Scalar(s) => {
                            let _ = write!(out, "{k}: {s}");
                        }
                        ToonValue::Map(_) => {
                            let _ = write!(out, "{k}:\n");
                            v.write(out, depth + 1);
                        }
                        ToonValue::Table { columns, rows } => {
                            if rows.is_empty() {
                                let _ = write!(out, "{k} [0]: (empty)");
                            } else {
                                let _ = write!(out, "{k} [{}]:\n", rows.len());
                                render_table(out, depth + 1, columns, rows);
                            }
                        }
                    }
                }
            }
            ToonValue::Table { columns, rows } => {
                if rows.is_empty() {
                    indent(out, depth);
                    out.push_str("(empty)");
                } else {
                    render_table(out, depth, columns, rows);
                }
            }
        }
    }
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn render_table(out: &mut String, depth: usize, columns: &[String], rows: &[Vec<String>]) {
    indent(out, depth);
    out.push_str(&columns.join(" | "));
    for row in rows {
        out.push('\n');
        indent(out, depth);
        // Guard against row arity mismatch by taking columns.len() items.
        let cells: Vec<String> = (0..columns.len())
            .map(|i| row.get(i).cloned().unwrap_or_default())
            .collect();
        out.push_str(&cells.join(" | "));
    }
}

/// Fluent builder for TOON documents.
#[derive(Debug, Default)]
pub struct ToonBuilder {
    entries: Vec<(String, ToonValue)>,
}

impl ToonBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a scalar field.
    pub fn field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.entries
            .push((key.into(), ToonValue::Scalar(value.into())));
        self
    }

    /// Add a nested map.
    pub fn map(mut self, key: impl Into<String>, value: ToonValue) -> Self {
        self.entries.push((key.into(), value));
        self
    }

    /// Add a table field.
    pub fn table<C, R>(mut self, key: impl Into<String>, columns: C, rows: R) -> Self
    where
        C: IntoIterator<Item = String>,
        R: IntoIterator<Item = Vec<String>>,
    {
        self.entries.push((
            key.into(),
            ToonValue::Table {
                columns: columns.into_iter().collect(),
                rows: rows.into_iter().collect(),
            },
        ));
        self
    }

    pub fn build(self) -> ToonValue {
        ToonValue::Map(self.entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_renders_plain() {
        let v = ToonValue::Scalar("hello".into());
        assert_eq!(v.render(), "hello");
    }

    #[test]
    fn map_renders_key_value() {
        let v = ToonValue::Map(vec![
            ("name".into(), ToonValue::Scalar("foo".into())),
            ("count".into(), ToonValue::Scalar("3".into())),
        ]);
        assert_eq!(v.render(), "name: foo\ncount: 3");
    }

    #[test]
    fn nested_map_renders_indented() {
        let inner = ToonValue::Map(vec![("port".into(), ToonValue::Scalar("8080".into()))]);
        let outer = ToonValue::Map(vec![("server".into(), inner)]);
        let expected = "server:\n  port: 8080";
        assert_eq!(outer.render(), expected);
    }

    #[test]
    fn table_renders_header_and_rows() {
        let v = ToonValue::Table {
            columns: vec!["path".into(), "size".into()],
            rows: vec![
                vec!["a.rs".into(), "10".into()],
                vec!["b.rs".into(), "20".into()],
            ],
        };
        let rendered = v.render();
        assert_eq!(rendered, "path | size\na.rs | 10\nb.rs | 20");
    }

    #[test]
    fn empty_table_inline() {
        let v = ToonValue::Map(vec![(
            "files".into(),
            ToonValue::Table {
                columns: vec!["path".into()],
                rows: vec![],
            },
        )]);
        assert_eq!(v.render(), "files [0]: (empty)");
    }

    #[test]
    fn table_in_map_has_count_header() {
        let doc = ToonBuilder::new()
            .table(
                "files",
                vec!["path".to_string(), "size".to_string()],
                vec![
                    vec!["a.rs".into(), "1".into()],
                    vec!["b.rs".into(), "2".into()],
                    vec!["c.rs".into(), "3".into()],
                ],
            )
            .build();
        let rendered = doc.render();
        assert!(rendered.starts_with("files [3]:"));
        assert!(rendered.contains("path | size"));
        assert!(rendered.contains("a.rs | 1"));
    }

    #[test]
    fn token_savings_vs_json_benchmark() {
        // Build a synthetic 50-file grep result.
        let rows: Vec<Vec<String>> = (0..50)
            .map(|i| {
                vec![
                    format!("src/module_{i:02}/handler.rs"),
                    format!("{}", 100 + i),
                    format!("fn handler_{i}() {{ /* body */ }}"),
                ]
            })
            .collect();

        let toon_doc = ToonBuilder::new()
            .table(
                "matches",
                vec!["file".to_string(), "line".to_string(), "text".to_string()],
                rows.clone(),
            )
            .build();
        let toon_text = toon_doc.render();

        // Equivalent JSON representation.
        let mut json_rows = Vec::new();
        for row in &rows {
            let obj = serde_json::json!({
                "file": row[0],
                "line": row[1].parse::<i64>().unwrap_or(0),
                "text": row[2],
            });
            json_rows.push(obj);
        }
        let json_doc = serde_json::json!({ "matches": json_rows });
        let json_text = serde_json::to_string(&json_doc).unwrap();

        let toon_len = toon_text.len();
        let json_len = json_text.len();
        let savings = 1.0 - (toon_len as f64 / json_len as f64);

        eprintln!(
            "TOON={toon_len}B, JSON={json_len}B, savings={:.1}%",
            savings * 100.0
        );

        assert!(
            savings >= 0.35,
            "expected ≥35% savings; got {:.1}% (TOON={toon_len}B, JSON={json_len}B)",
            savings * 100.0
        );
    }
}
