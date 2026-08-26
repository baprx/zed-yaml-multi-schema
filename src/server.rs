//! Language-server facade: ties the document model, resolver, validator, and
//! completion together into per-block diagnostics/completions. Testable against
//! a fake fetcher without the WASM host.

use std::path::Path;

use crate::completion;
use crate::document::Document;
use crate::resolver::{ResolveOutcome, SchemaFetcher, SchemaResolver};

/// A diagnostic targeting a governed block.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// 0-based start line.
    pub start_line: usize,
    /// 0-based end line (inclusive).
    pub end_line: usize,
    /// Message.
    pub message: String,
    /// `error`, `warning`, or `info`.
    pub severity: String,
}

/// A completion item surfaced to the editor.
#[derive(Debug, Clone)]
pub struct Completion {
    pub label: String,
    pub kind: i32,
    pub detail: Option<String>,
    /// Optional snippet text; when present the caller emits it as `insertText`
    /// with `insertTextFormat` Snippet (2).
    pub insert_text: Option<String>,
}

/// Holds the current state for one open document and produces diagnostics and
/// completions scoped to annotated blocks.
pub struct YamlServer<'a> {
    resolver: SchemaResolver<'a>,
    document: Document,
    /// Cached validation diagnostics for the current document.
    diagnostics: Vec<Diagnostic>,
}

impl<'a> YamlServer<'a> {
    pub fn new(fetcher: &'a dyn SchemaFetcher, worktree_root: &'a Path) -> Self {
        Self {
            resolver: SchemaResolver::new(fetcher, worktree_root),
            document: Document::default(),
            diagnostics: Vec::new(),
        }
    }

    /// Handles a document open/change: reparses, resolves schemas, validates
    /// governed blocks, and recomputes scoped diagnostics.
    pub fn on_change(&mut self, text: &str) {
        let doc = match Document::parse(text) {
            Ok(doc) => doc,
            Err(_) => {
                // If the document is not valid YAML, drop state and clear
                // diagnostics; the editor stays usable.
                self.document = Document::default();
                self.diagnostics = Vec::new();
                return;
            }
        };

        let mut diagnostics = Vec::new();
        let lines: Vec<&str> = text.lines().collect();
        for block in &doc.blocks {
            let Some(reference) = block.schema_ref.as_deref() else {
                // Unannotated block: no governing schema, no diagnostics.
                continue;
            };
            match self.resolver.resolve(reference) {
                ResolveOutcome::Resolved { schema } => {
                    match crate::validator::validate(&schema, &block.value) {
                        Ok(findings) => {
                            for finding in findings {
                                // Point the diagnostic at the specific offending
                                // key/element line rather than the whole block.
                                let segs: Vec<String> = finding
                                    .instance_path
                                    .trim_start_matches('/')
                                    .split('/')
                                    .filter(|s| !s.is_empty())
                                    .map(|s| s.to_string())
                                    .collect();
                                let line =
                                    crate::document::line_for_path(&lines, block.start_line, &segs);
                                diagnostics.push(Diagnostic {
                                    start_line: line,
                                    end_line: line,
                                    message: format!(
                                        "{}: {}",
                                        finding.instance_path, finding.message
                                    ),
                                    severity: "error".to_string(),
                                });
                            }
                        }
                        Err(reason) => diagnostics.push(Diagnostic {
                            start_line: block.start_line,
                            end_line: block.end_line,
                            message: reason,
                            severity: "error".to_string(),
                        }),
                    }
                }
                ResolveOutcome::Failed { reason } => {
                    diagnostics.push(Diagnostic {
                        start_line: block.start_line,
                        end_line: block.end_line,
                        message: reason,
                        severity: "warning".to_string(),
                    });
                }
            }
        }

        self.document = doc;
        self.diagnostics = diagnostics;
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Returns completions for the block at the given 0-based line and column,
    /// based on the cursor context (key vs value position). Only annotated,
    /// resolvable blocks yield completions.
    pub fn complete_at(&mut self, text: &str, line: usize, character: usize) -> Vec<Completion> {
        let Some(block) = self.document.block_at_line(line) else {
            return Vec::new();
        };
        let Some(reference) = block.schema_ref.as_deref() else {
            return Vec::new();
        };
        let ResolveOutcome::Resolved { schema } = self.resolver.resolve(reference) else {
            return Vec::new();
        };
        let lines: Vec<&str> = text.lines().collect();
        let (path, position) = crate::document::context(&lines, block.start_line, line, character);
        let cursor_indent = lines
            .get(line)
            .map(|l| l.len() - l.trim_start().len())
            .unwrap_or(0);
        let existing_keys = existing_keys(&block.value, &path);
        completion::complete(&schema, &path, position, cursor_indent + 2, &existing_keys)
            .into_iter()
            .map(|i| Completion {
                label: i.label,
                kind: i.kind,
                detail: i.detail,
                insert_text: i.insert_text,
            })
            .collect()
    }
}

/// Collects the property names already present in the mapping at `path`
/// (relative to a block's value), so key completions can avoid duplicates.
fn existing_keys(value: &serde_yaml::Value, path: &[String]) -> Vec<String> {
    use serde_yaml::Value;
    let mut node = value;
    for key in path {
        match node {
            Value::Mapping(map) => match map.get(Value::String(key.clone())) {
                Some(next) => node = next,
                None => return Vec::new(),
            },
            _ => return Vec::new(),
        }
    }
    match node {
        Value::Mapping(map) => map
            .keys()
            .filter_map(|k| k.as_str().map(String::from))
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct FakeFetcher {
        local: HashMap<String, String>,
    }

    impl SchemaFetcher for FakeFetcher {
        fn read_local(&self, path: &str) -> Result<String, String> {
            self.local
                .get(path)
                .cloned()
                .ok_or_else(|| format!("no local file '{path}'"))
        }
        fn fetch_remote(&self, _url: &str) -> Result<String, String> {
            Err("network disabled in tests".to_string())
        }
    }

    fn fetcher_with_schema() -> FakeFetcher {
        let mut local = HashMap::new();
        local.insert(
            "/root/schemas/test.schema.json".to_string(),
            r#"{"type":"object","properties":{"enabled":{"type":"boolean"}},"required":["enabled"]}"#
                .to_string(),
        );
        FakeFetcher { local }
    }

    #[test]
    fn diagnostics_scoped_to_annotated_block() {
        let fetcher = fetcher_with_schema();
        let mut server = YamlServer::new(&fetcher, Path::new("/root"));
        server.on_change(
            "# $schema=./schemas/test.schema.json\ntest:\n  enabled: not-a-bool\n\ntraefik:\n  image:\n    tag: 1\n",
        );
        let diags = server.diagnostics();
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("enabled"));
        assert_eq!(diags[0].start_line, 2);
    }

    #[test]
    fn no_diagnostics_for_unannotated_block() {
        let fetcher = fetcher_with_schema();
        let mut server = YamlServer::new(&fetcher, Path::new("/root"));
        server.on_change("kubernetes:\n  kind: ConfigMap\n");
        assert!(server.diagnostics().is_empty());
    }

    #[test]
    fn multiple_blocks_isolated() {
        let fetcher = fetcher_with_schema();
        let mut server = YamlServer::new(&fetcher, Path::new("/root"));
        // Only `test` is annotated; `traefik` is unannotated.
        server.on_change(
            "# $schema=./schemas/test.schema.json\ntest:\n  enabled: bad\n\ntraefik:\n  anything: at all\n",
        );
        let diags = server.diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].start_line, 2);
    }

    #[test]
    fn unreachable_schema_yields_warning_and_keeps_editable() {
        let fetcher = FakeFetcher {
            local: HashMap::new(),
        };
        let mut server = YamlServer::new(&fetcher, Path::new("/root"));
        server.on_change("# $schema=./missing.json\nfoo:\n  bar: 1\n");
        let diags = server.diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, "warning");
        assert!(diags[0].message.contains("missing.json"));
    }

    #[test]
    fn completion_only_in_governed_block() {
        let fetcher = fetcher_with_schema();
        let mut server = YamlServer::new(&fetcher, Path::new("/root"));
        let text = "# $schema=./schemas/test.schema.json\ntest:\n  enabled: \n\nother:\n  x: 1\n";
        server.on_change(text);
        // Cursor after `enabled:` (value position) inside `test` yields completions.
        let in_block = server.complete_at(text, 2, 12);
        assert!(!in_block.is_empty());
        // Cursor inside unannotated `other` block yields none.
        let other = server.complete_at(text, 5, 4);
        assert!(other.is_empty());
    }

    #[test]
    fn completion_excludes_keys_already_in_map() {
        let mut local = HashMap::new();
        local.insert(
            "/root/schemas/test.schema.json".to_string(),
            r#"{"type":"object","properties":{"enabled":{"type":"boolean"},"version":{"type":"number"}}}"#
                .to_string(),
        );
        let fetcher = FakeFetcher { local };
        let mut server = YamlServer::new(&fetcher, Path::new("/root"));
        // `enabled` is already present in the `test` map, so it must not be
        // suggested; `version` is absent and still offered. The trailing `extra`
        // key keeps the cursor line inside the block (blank lines are trimmed).
        let text = "# $schema=./schemas/test.schema.json\ntest:\n  enabled: true\n  \n  extra: x\n";
        server.on_change(text);
        let items = server.complete_at(text, 3, 4);
        let labels: Vec<String> = items.iter().map(|c| c.label.clone()).collect();
        assert!(!labels.contains(&"enabled".to_string()));
        assert!(labels.contains(&"version".to_string()));
    }

    #[test]
    fn completion_value_position_suggests_boolean() {
        let fetcher = fetcher_with_schema();
        let mut server = YamlServer::new(&fetcher, Path::new("/root"));
        let text = "# $schema=./schemas/test.schema.json\ntest:\n  enabled: \n";
        server.on_change(text);
        // Cursor after `enabled:` (value position) suggests booleans.
        let items = server.complete_at(text, 2, 12);
        let labels: Vec<String> = items.iter().map(|c| c.label.clone()).collect();
        assert_eq!(labels, vec!["true".to_string(), "false".to_string()]);
    }

    #[test]
    fn diagnostics_point_at_offending_key_line() {
        let fetcher = fetcher_with_schema();
        let mut server = YamlServer::new(&fetcher, Path::new("/root"));
        // `enabled` is on line 2; the diagnostic must target that line, not the
        // whole block.
        server.on_change(
            "# $schema=./schemas/test.schema.json\ntest:\n  enabled: not-a-bool\n  extra: 1\n",
        );
        let diags = server.diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].start_line, 2);
        assert_eq!(diags[0].end_line, 2);
    }
}
