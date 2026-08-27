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

/// Locates, without YAML parsing, the top-level block containing `line`: the
/// last top-level key at or above `line`, plus the `# $schema=` annotation
/// directly above that key (only blank/comment lines in between), if any.
/// Used when the document cannot be parsed — e.g. a key typed before its
/// colon — so completions keep working from the raw lines.
pub fn block_span_at_line(lines: &[&str], line: usize) -> Option<(usize, Option<String>)> {
    if line >= lines.len() {
        return None;
    }
    let key_line = (0..=line)
        .rev()
        .find(|&i| is_top_level_key_line(lines[i]))?;
    let mut schema_ref = None;
    for i in (0..key_line).rev() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            if schema_ref.is_none() {
                schema_ref = parse_annotation(lines[i]);
            }
            continue;
        }
        break;
    }
    Some((key_line, schema_ref))
}

/// Collects, without YAML parsing, the mapping keys at `cursor_indent` within
/// the block starting at `start_line` (up to the next top-level key), for
/// duplicate filtering while the document is temporarily invalid. Partial
/// keys being typed (no colon yet) are not mapping keys and are excluded.
pub fn sibling_keys(lines: &[&str], start_line: usize, cursor_indent: usize) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for line in lines.iter().skip(start_line + 1) {
        if is_top_level_key_line(line) {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("- ") {
            continue;
        }
        let indent = line.len() - trimmed.len();
        if indent != cursor_indent {
            continue;
        }
        if let Some((key, _)) = trimmed.split_once(':') {
            let key = key.trim().trim_matches(['"', '\'']);
            if !key.is_empty() && !keys.iter().any(|k| k == key) {
                keys.push(key.to_string());
            }
        }
    }
    keys
}

/// Whether the cursor is at a key position (choosing a property name) or a
/// value position (choosing a value after `key:`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorPosition {
    Key,
    Value,
    /// The cursor sits at an indentation that cannot be a child of the
    /// enclosing map (e.g. a key at column 0 inside a block), so no completion
    /// should be offered.
    Invalid,
}

/// Computes the completion context at the cursor within a block:
/// the enclosing mapping-key path (ancestors above `cursor_line`, excluding the
/// top-level block key at `start_line`) and whether the cursor is at a key or
/// value position.
pub fn context(
    lines: &[&str],
    start_line: usize,
    cursor_line: usize,
    cursor_col: usize,
) -> (Vec<String>, CursorPosition) {
    let cur = lines.get(cursor_line).copied().unwrap_or("");
    let cursor_indent = cur.len() - cur.trim_start().len();

    let mut stack: Vec<(usize, String)> = Vec::new();
    for line in lines.iter().take(cursor_line).skip(start_line + 1) {
        push_mapping_key(line, &mut stack);
    }
    // Keys at the cursor's own indentation (or deeper) are siblings or
    // descendants of the cursor, not ancestors; drop them so the enclosing path
    // reflects the map the cursor actually belongs to.
    while let Some(&(si, _)) = stack.last() {
        if si >= cursor_indent {
            stack.pop();
        } else {
            break;
        }
    }
    let parent_indent = stack
        .last()
        .map(|&(si, _)| si)
        .unwrap_or_else(|| indent_of(lines[start_line]));
    let expected_child_indent = parent_indent + 2;
    let enclosing: Vec<String> = stack.into_iter().map(|(_, k)| k).collect();

    let trimmed = cur.trim();
    if !trimmed.is_empty() && !trimmed.starts_with('#') {
        if let Some((key, _)) = trimmed.split_once(':') {
            let key = key.trim().trim_matches(['"', '\'']);
            if !key.is_empty() {
                let colon = cur.find(':').unwrap_or(0);
                if colon < cursor_col {
                    let mut path = enclosing.clone();
                    path.push(key.to_string());
                    return (path, CursorPosition::Value);
                }
            }
        }
    }
    if cursor_indent < expected_child_indent {
        return (Vec::new(), CursorPosition::Invalid);
    }
    (enclosing, CursorPosition::Key)
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Maps a JSON-pointer-style path (relative to a block value) to the 0-based
/// line of the offending key or array element within the block's source.
///
/// `path` is the split pointer (e.g. `/elements/2` -> `["elements", "2"]`).
/// Mapping keys are located at their indentation level; numeric segments count
/// `- ` list items. Returns the block's start line when the path is empty or
/// cannot be resolved (best-effort).
pub fn line_for_path(lines: &[&str], block_start_line: usize, path: &[String]) -> usize {
    let mut container_indent = indent_of(lines[block_start_line]) + 2;
    let mut cursor = block_start_line;
    for seg in path {
        let from = cursor + 1;
        let line = match seg.parse::<usize>() {
            Ok(idx) => item_line(lines, from, container_indent, idx),
            Err(_) => key_line(lines, from, container_indent, seg),
        };
        match line {
            Some(l) => {
                cursor = l;
                container_indent += 2;
            }
            None => return block_start_line,
        }
    }
    cursor
}

/// Finds the line at or after `from` where the mapping key `key` appears at
/// exactly `container_indent`. Stops (returns None) once a shallower-indented
/// line is reached, meaning we left the containing mapping.
fn key_line(lines: &[&str], from: usize, container_indent: usize, key: &str) -> Option<usize> {
    for (i, line) in lines.iter().enumerate().skip(from) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = indent_of(line);
        if indent < container_indent {
            return None;
        }
        if indent == container_indent {
            if let Some((k, _)) = trimmed.split_once(':') {
                if k.trim().trim_matches(['"', '\'']) == key {
                    return Some(i);
                }
            }
        }
    }
    None
}

/// Finds the line of the `index`-th `- ` list item at `container_indent`,
/// scanning from `from` onward and stopping once a shallower line is reached.
fn item_line(lines: &[&str], from: usize, container_indent: usize, index: usize) -> Option<usize> {
    let mut seen = 0usize;
    for (i, line) in lines.iter().enumerate().skip(from) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = indent_of(line);
        if indent < container_indent {
            return None;
        }
        if indent == container_indent && trimmed.starts_with("- ") {
            if seen == index {
                return Some(i);
            }
            seen += 1;
        }
    }
    None
}

/// Pushes a mapping key from `line` onto the indent-based path stack, popping
/// siblings nested no deeper than this line.
fn push_mapping_key(line: &str, stack: &mut Vec<(usize, String)>) {
    let indent = line.len() - line.trim_start().len();
    let trimmed = line.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with("---")
        || trimmed.starts_with("...")
    {
        return;
    }
    if let Some(rest) = trimmed.strip_prefix("- ") {
        if let Some((key, _)) = rest.split_once(':') {
            let key = key.trim().trim_matches(['"', '\'']);
            if key.is_empty() {
                return;
            }
            let eff = indent + 2;
            while let Some(&(si, _)) = stack.last() {
                if si >= eff {
                    stack.pop();
                } else {
                    break;
                }
            }
            stack.push((eff, key.to_string()));
        }
        return;
    }
    if let Some((key, _)) = trimmed.split_once(':') {
        let key = key.trim().trim_matches(['"', '\'']);
        if key.is_empty() {
            return;
        }
        while let Some(&(si, _)) = stack.last() {
            if si >= indent {
                stack.pop();
            } else {
                break;
            }
        }
        stack.push((indent, key.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_span_at_line_finds_key_and_annotation() {
        let text =
            "# $schema=./schemas/test.schema.json\ntest:\n  enabled: true\n  im\n  product: r\n";
        let lines: Vec<&str> = text.lines().collect();
        let (key_line, schema_ref) = block_span_at_line(&lines, 3).expect("block found");
        assert_eq!(key_line, 1);
        assert_eq!(schema_ref.as_deref(), Some("./schemas/test.schema.json"));
    }

    #[test]
    fn block_span_at_line_none_above_first_key() {
        let text = "# $schema=./schemas/test.schema.json\ntest:\n";
        let lines: Vec<&str> = text.lines().collect();
        assert!(block_span_at_line(&lines, 0).is_none());
    }

    #[test]
    fn sibling_keys_collect_indent_matches_across_cursor() {
        let text = "test:\n  enabled: true\n  elements:\n    - 1\n  im\n  product: r\n";
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            sibling_keys(&lines, 0, 2),
            vec![
                "enabled".to_string(),
                "elements".to_string(),
                "product".to_string()
            ]
        );
    }

    #[test]
    fn sibling_keys_stop_at_next_top_level_key() {
        let text = "test:\n  enabled: true\n\ntraefik:\n  enabled: false\n";
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(sibling_keys(&lines, 0, 2), vec!["enabled".to_string()]);
    }

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

    #[test]
    fn context_value_position_at_nested_key() {
        let text = "traefik:\n  image:\n    registry: ex\n";
        let lines: Vec<&str> = text.lines().collect();
        // Cursor after `registry:` on line 2.
        let (path, pos) = context(&lines, 0, 2, 17);
        assert_eq!(path, vec!["image".to_string(), "registry".to_string()]);
        assert_eq!(pos, CursorPosition::Value);
    }

    #[test]
    fn context_key_position_lists_enclosing_keys() {
        let text = "traefik:\n  image:\n    \n";
        let lines: Vec<&str> = text.lines().collect();
        // Empty line 2 under `image:` at key position.
        let (path, pos) = context(&lines, 0, 2, 4);
        assert_eq!(path, vec!["image".to_string()]);
        assert_eq!(pos, CursorPosition::Key);
    }

    #[test]
    fn context_key_position_editing_existing_key() {
        let text = "traefik:\n  image\n";
        let lines: Vec<&str> = text.lines().collect();
        // Cursor on `image` (no colon yet) at key position.
        let (path, pos) = context(&lines, 0, 1, 6);
        assert_eq!(path, Vec::<String>::new());
        assert_eq!(pos, CursorPosition::Key);
    }

    #[test]
    fn context_cursor_after_sibling_key_belongs_to_parent_map() {
        // Cursor on an empty line under `image:` but after a sibling `pullPolicy`
        // must still resolve to the `image` map, not `pullPolicy`.
        let text = "test:\n  image:\n    pullPolicy: Always\n    \n";
        let lines: Vec<&str> = text.lines().collect();
        let (path, pos) = context(&lines, 0, 3, 4);
        assert_eq!(path, vec!["image".to_string()]);
        assert_eq!(pos, CursorPosition::Key);
    }

    #[test]
    fn context_cursor_deeper_than_sibling_nests_inside_object() {
        let text = "test:\n  image:\n    pullPolicy: Always\n      \n";
        let lines: Vec<&str> = text.lines().collect();
        // Cursor at indent 6 (deeper than pullPolicy) nests inside pullPolicy.
        let (path, _) = context(&lines, 0, 3, 6);
        assert_eq!(path, vec!["image".to_string(), "pullPolicy".to_string()]);
    }

    #[test]
    fn context_rejects_too_shallow_key_indentation() {
        // `test`'s children must be at indent 2; a cursor at indent 0 is invalid.
        let text = "test:\n  enabled: true\n  elements:\n    - 1\n    - A\n\n  product: r\n";
        let lines: Vec<&str> = text.lines().collect();
        let (path, pos) = context(&lines, 0, 5, 0);
        assert_eq!(path, Vec::<String>::new());
        assert_eq!(pos, CursorPosition::Invalid);
    }

    #[test]
    fn context_accepts_valid_child_indentation() {
        let text = "test:\n  enabled: true\n  \n";
        let lines: Vec<&str> = text.lines().collect();
        let (_, pos) = context(&lines, 0, 2, 2);
        assert_eq!(pos, CursorPosition::Key);
    }

    #[test]
    fn line_for_path_maps_top_level_key() {
        let text = "test:\n  enabled: not-a-bool\n  version: 1\n";
        let lines: Vec<&str> = text.lines().collect();
        let line = line_for_path(&lines, 0, &["enabled".to_string()]);
        assert_eq!(line, 1);
    }

    #[test]
    fn line_for_path_maps_nested_key() {
        let text = "test:\n  image:\n    pullPolicy: Always\n    tag: bad\n";
        let lines: Vec<&str> = text.lines().collect();
        let line = line_for_path(&lines, 0, &["image".to_string(), "tag".to_string()]);
        assert_eq!(line, 3);
    }

    #[test]
    fn line_for_path_maps_array_element() {
        let text = "test:\n  elements:\n    - one\n    - two\n";
        let lines: Vec<&str> = text.lines().collect();
        let line = line_for_path(&lines, 0, &["elements".to_string(), "1".to_string()]);
        assert_eq!(line, 3);
    }

    #[test]
    fn line_for_path_falls_back_to_block_start() {
        let text = "test:\n  enabled: true\n";
        let lines: Vec<&str> = text.lines().collect();
        // Unresolvable path falls back to the block's own line.
        let line = line_for_path(&lines, 0, &["ghost".to_string()]);
        assert_eq!(line, 0);
    }
}
