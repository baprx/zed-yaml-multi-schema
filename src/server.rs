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
    pub kind: String,
    pub detail: Option<String>,
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
                                diagnostics.push(Diagnostic {
                                    start_line: block.start_line,
                                    end_line: block.end_line,
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

    /// Returns completions for the block at the given 0-based line. Only
    /// annotated, resolvable blocks yield completions.
    pub fn complete_at_line(&mut self, line: usize) -> Vec<Completion> {
        let Some(block) = self.document.block_at_line(line) else {
            return Vec::new();
        };
        let Some(reference) = block.schema_ref.as_deref() else {
            return Vec::new();
        };
        let ResolveOutcome::Resolved { schema } = self.resolver.resolve(reference) else {
            return Vec::new();
        };
        completion::complete(&schema, 0)
            .into_iter()
            .map(|i| Completion {
                label: i.label,
                kind: i.kind,
                detail: i.detail,
            })
            .collect()
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
        assert_eq!(diags[0].start_line, 1);
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
        assert_eq!(diags[0].start_line, 1);
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
        server.on_change(
            "# $schema=./schemas/test.schema.json\ntest:\n  enabled: true\n\nother:\n  x: 1\n",
        );
        // Cursor inside `test` block yields completions.
        let in_block = server.complete_at_line(2);
        assert!(in_block.iter().any(|c| c.label == "enabled"));
        // Cursor inside unannotated `other` block yields none.
        let other = server.complete_at_line(5);
        assert!(other.is_empty());
    }
}
