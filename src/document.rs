//! Document model: parsing YAML and mapping `# $schema=` annotations to blocks.

use serde_yaml::Value;

/// The reference extracted from a `# $schema=<ref>` annotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annotation {
    /// The raw reference string (local path or remote HTTPS URL).
    pub reference: String,
    /// 0-based line index of the annotation comment.
    pub line: usize,
}

/// A top-level block in the document.
#[derive(Debug, Clone)]
pub struct Block {
    /// The top-level mapping key (e.g. `traefik`, `argocd-apps`).
    pub key: String,
    /// The structured YAML value of the block (the mapping/scalar under `key`).
    pub value: Value,
    /// 0-based start line of the block (the key line).
    pub start_line: usize,
    /// 0-based end line (inclusive) of the block body.
    pub end_line: usize,
    /// The governing schema reference, if the block is annotated.
    pub schema_ref: Option<String>,
}

/// A parsed YAML document with per-block annotation mapping.
#[derive(Debug, Default)]
pub struct Document {
    pub blocks: Vec<Block>,
}

/// The `# $schema=` marker used to annotate a block.
const SCHEMA_MARKER: &str = "$schema=";

/// Returns true if `line` is a top-level key line (column 0, not a doc marker,
/// comment, empty, or list item).
fn is_top_level_key_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || trimmed.starts_with("---")
        || trimmed.starts_with("...")
        || trimmed.starts_with('#')
        || trimmed.starts_with('-')
    {
        return false;
    }
    !line.starts_with(char::is_whitespace)
}

/// Parses the annotation reference out of a comment line, if present.
fn parse_annotation(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with('#') {
        return None;
    }
    let idx = trimmed.find(SCHEMA_MARKER)?;
    let after = &trimmed[idx + SCHEMA_MARKER.len()..];
    let reference = after.trim().trim_matches(['"', '\'']).trim().to_string();
    if reference.is_empty() {
        None
    } else {
        Some(reference)
    }
}

impl Document {
    /// Parses a YAML document, mapping each `# $schema=` annotation to the
    /// top-level block that immediately follows it.
    pub fn parse(text: &str) -> Result<Document, String> {
        let lines: Vec<&str> = text.lines().collect();

        // Top-level key lines, in order.
        let key_lines: Vec<usize> = (0..lines.len())
            .filter(|&i| is_top_level_key_line(lines[i]))
            .collect();

        // Annotation comment lines with their reference.
        let annotations: Vec<Annotation> = (0..lines.len())
            .filter_map(|i| {
                parse_annotation(lines[i]).map(|reference| Annotation { reference, line: i })
            })
            .collect();

        // Root value for reading block values.
        let root: Value =
            serde_yaml::from_str(text).map_err(|e| format!("YAML parse error: {e}"))?;

        let mut blocks = Vec::new();

        for (pos, &key_line) in key_lines.iter().enumerate() {
            let mut end_line = key_lines
                .get(pos + 1)
                .map(|&next| next.saturating_sub(1))
                .unwrap_or(lines.len().saturating_sub(1));

            // Trim trailing blank/comment lines so the block range covers only
            // its own content (keeps diagnostics from including separators).
            while end_line > key_line {
                let trimmed = lines[end_line].trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    end_line -= 1;
                } else {
                    break;
                }
            }

            // The key is the text before the first ':' on the key line.
            let raw = lines[key_line];
            let key = raw
                .split_once(':')
                .map(|(k, _)| k.trim().trim_matches(['"', '\'']).to_string())
                .unwrap_or_default();

            // Attach the governing annotation if one appears directly above this
            // key line (only blank/comment lines in between).
            let schema_ref = annotations
                .iter()
                .find(|a| {
                    a.line < key_line
                        && (key_line - a.line) >= 1
                        && lines[a.line + 1..key_line]
                            .iter()
                            .all(|l| l.trim().is_empty() || l.trim().starts_with('#'))
                })
                .map(|a| a.reference.clone());

            let value = match &root {
                Value::Mapping(map) => map
                    .get(Value::String(key.clone()))
                    .cloned()
                    .unwrap_or(Value::Null),
                _ => Value::Null,
            };

            blocks.push(Block {
                key,
                value,
                start_line: key_line,
                end_line,
                schema_ref,
            });
        }

        Ok(Document { blocks })
    }

    /// Returns the block containing the given 0-based line, if any.
    pub fn block_at_line(&self, line: usize) -> Option<&Block> {
        self.blocks
            .iter()
            .find(|b| line >= b.start_line && line <= b.end_line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_annotation_to_block() {
        let text = "---\n# $schema=./schemas/test.schema.json\ntest:\n  enabled: true\n\ntraefik:\n  image:\n    tag: 1.1.1\n";
        let doc = Document::parse(text).unwrap();
        assert_eq!(doc.blocks.len(), 2);
        assert_eq!(doc.blocks[0].key, "test");
        assert_eq!(
            doc.blocks[0].schema_ref.as_deref(),
            Some("./schemas/test.schema.json")
        );
        assert!(doc.blocks[1].schema_ref.is_none());
    }

    #[test]
    fn block_line_ranges() {
        let text = "test:\n  enabled: true\n  version: 1\n\ntraefik:\n  image:\n    tag: 1\n";
        let doc = Document::parse(text).unwrap();
        assert_eq!(doc.blocks[0].start_line, 0);
        assert_eq!(doc.blocks[0].end_line, 2);
        assert_eq!(doc.blocks[1].start_line, 4);
        assert_eq!(doc.blocks[1].end_line, 6);
    }

    #[test]
    fn unannotated_blocks_have_no_schema() {
        let text = "kubernetes:\n  kind: ConfigMap\n";
        let doc = Document::parse(text).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(doc.blocks[0].schema_ref.is_none());
    }
}
