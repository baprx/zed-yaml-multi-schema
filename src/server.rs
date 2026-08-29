//! Language-server facade: ties the document model, resolver, validator, and
//! completion together into per-block diagnostics/completions. Testable against
//! a fake fetcher without the WASM host.

use std::path::Path;

use crate::completion;
use crate::document::Document;
use crate::resolver::{ResolveOutcome, SchemaFetcher, SchemaResolver};
use std::sync::Arc;

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
    /// Optional `insertText`. When present, the caller emits it as `insertText`
    /// with the format given by `insert_text_format`.
    pub insert_text: Option<String>,
    /// LSP `InsertTextFormat` (1 = PlainText, 2 = Snippet) to use with
    /// `insert_text`, when present.
    pub insert_text_format: Option<i32>,
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
    pub fn new(fetcher: Arc<dyn SchemaFetcher>, worktree_root: &'a Path) -> Self {
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
                // Mid-edit documents are often temporarily invalid YAML (e.g.
                // a key typed before its colon). Keep the last good parse so
                // completions keep working; diagnostics are cleared until the
                // document parses again.
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
                    match crate::validator::validate(&schema, &block.value, self.resolver.fetcher())
                    {
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
        let lines: Vec<&str> = text.lines().collect();
        // Prefer the parsed document; when it has no block for the cursor
        // (document unparsed or temporarily invalid YAML), fall back to a
        // scan of the raw lines so typing a key name still completes.
        let (start_line, reference, value) = match self.document.block_at_line(line) {
            Some(block) => (
                block.start_line,
                block.schema_ref.clone(),
                Some(block.value.clone()),
            ),
            None => match crate::document::block_span_at_line(&lines, line) {
                Some((key_line, Some(reference))) => (key_line, Some(reference), None),
                _ => return Vec::new(),
            },
        };
        let Some(reference) = reference else {
            return Vec::new();
        };
        let ResolveOutcome::Resolved { schema } = self.resolver.resolve(&reference) else {
            return Vec::new();
        };
        let (path, position) = crate::document::context(&lines, start_line, line, character);
        let cursor_indent = lines
            .get(line)
            .map(|l| l.len() - l.trim_start().len())
            .unwrap_or(0);
        let existing_keys = match value {
            Some(value) => existing_keys(&value, &path),
            // Sibling keys derived from the raw lines are only a faithful
            // duplicate filter at the block's top level; deeper paths skip it.
            None if path.is_empty() => {
                crate::document::sibling_keys(&lines, start_line, cursor_indent)
            }
            None => Vec::new(),
        };
        completion::complete(&schema, &path, position, cursor_indent + 2, &existing_keys)
            .into_iter()
            .map(|i| Completion {
                label: i.label,
                kind: i.kind,
                detail: i.detail,
                insert_text: i.insert_text,
                insert_text_format: i.insert_text_format,
            })
            .collect()
    }
}

/// Collects the property names already present in the mapping at `path`
/// (relative to a block's value), so key completions can avoid duplicates.
fn existing_keys(value: &yaml_serde::Value, path: &[String]) -> Vec<String> {
    use yaml_serde::Value;
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

    /// Schema mirroring `schemas/test.schema.json`: several root properties so
    /// key-completion tests can assert which names are suggested.
    fn fetcher_with_full_schema() -> FakeFetcher {
        let mut local = HashMap::new();
        local.insert(
            "/root/schemas/test.schema.json".to_string(),
            r#"{"type":"object","properties":{"enabled":{"type":"boolean"},"elements":{"type":"array"},"product":{"type":"string"},"version":{"type":"number"},"image":{"type":"object"}},"required":["enabled"]}"#
                .to_string(),
        );
        FakeFetcher { local }
    }

    #[test]
    fn diagnostics_scoped_to_annotated_block() {
        let fetcher = fetcher_with_schema();
        let mut server = YamlServer::new(Arc::new(fetcher), Path::new("/root"));
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
        let mut server = YamlServer::new(Arc::new(fetcher), Path::new("/root"));
        server.on_change("kubernetes:\n  kind: ConfigMap\n");
        assert!(server.diagnostics().is_empty());
    }

    #[test]
    fn multiple_blocks_isolated() {
        let fetcher = fetcher_with_schema();
        let mut server = YamlServer::new(Arc::new(fetcher), Path::new("/root"));
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
        let mut server = YamlServer::new(Arc::new(fetcher), Path::new("/root"));
        server.on_change("# $schema=./missing.json\nfoo:\n  bar: 1\n");
        let diags = server.diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, "warning");
        assert!(diags[0].message.contains("missing.json"));
    }

    #[test]
    fn key_completion_carries_colon() {
        let fetcher = fetcher_with_schema();
        let mut server = YamlServer::new(Arc::new(fetcher), Path::new("/root"));
        let text = "# $schema=./schemas/test.schema.json\ntest:\n  \n  extra: x\n";
        server.on_change(text);
        let items = server.complete_at(text, 2, 2);
        let item = items
            .iter()
            .find(|c| c.label == "enabled")
            .expect("key present");
        assert_eq!(item.insert_text.as_deref(), Some("enabled: "));
    }

    #[test]
    fn completion_only_in_governed_block() {
        let fetcher = fetcher_with_schema();
        let mut server = YamlServer::new(Arc::new(fetcher), Path::new("/root"));
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
        let mut server = YamlServer::new(Arc::new(fetcher), Path::new("/root"));
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
    fn completion_empty_at_invalid_indentation() {
        let mut local = HashMap::new();
        local.insert(
            "/root/schemas/test.schema.json".to_string(),
            r#"{"type":"object","properties":{"enabled":{"type":"boolean"},"version":{"type":"number"}}}"#
                .to_string(),
        );
        let fetcher = FakeFetcher { local };
        let mut server = YamlServer::new(Arc::new(fetcher), Path::new("/root"));
        // Cursor at indent 0 (column 0) inside the `test` block is invalid.
        let text =
            "# $schema=./schemas/test.schema.json\ntest:\n  enabled: true\n  version: 1\n\n  product: r\n";
        server.on_change(text);
        let items = server.complete_at(text, 4, 0);
        assert!(items.is_empty());
    }

    #[test]
    fn completion_value_position_suggests_boolean() {
        let fetcher = fetcher_with_schema();
        let mut server = YamlServer::new(Arc::new(fetcher), Path::new("/root"));
        let text = "# $schema=./schemas/test.schema.json\ntest:\n  enabled: \n";
        server.on_change(text);
        // Cursor after `enabled:` (value position) suggests booleans.
        let items = server.complete_at(text, 2, 12);
        let labels: Vec<String> = items.iter().map(|c| c.label.clone()).collect();
        assert_eq!(labels, vec!["true".to_string(), "false".to_string()]);
    }

    #[test]
    fn completion_suggests_while_typing_partial_key() {
        let fetcher = fetcher_with_full_schema();
        let mut server = YamlServer::new(Arc::new(fetcher), Path::new("/root"));
        // `im` is typed without its colon, so the document is temporarily
        // invalid YAML; completions must still be offered from the raw lines.
        let text = "# $schema=./schemas/test.schema.json\ntest:\n  enabled: true\n  elements:\n    - 1\n    - A\n  im\n  product: r\n  version: 1\n";
        server.on_change(text);
        let items = server.complete_at(text, 6, 4);
        let labels: Vec<String> = items.iter().map(|c| c.label.clone()).collect();
        assert_eq!(labels, vec!["image".to_string()]);
    }

    #[test]
    fn completion_on_empty_line_with_siblings_below() {
        let fetcher = fetcher_with_full_schema();
        let mut server = YamlServer::new(Arc::new(fetcher), Path::new("/root"));
        let text = "# $schema=./schemas/test.schema.json\ntest:\n  enabled: true\n  elements:\n    - 1\n    - A\n  \n  product: r\n  version: 1\n";
        server.on_change(text);
        let items = server.complete_at(text, 6, 2);
        let labels: Vec<String> = items.iter().map(|c| c.label.clone()).collect();
        assert_eq!(labels, vec!["image".to_string()]);
    }

    #[test]
    fn diagnostics_point_at_offending_key_line() {
        let fetcher = fetcher_with_schema();
        let mut server = YamlServer::new(Arc::new(fetcher), Path::new("/root"));
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
