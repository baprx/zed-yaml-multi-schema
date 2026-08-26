//! Derive completion items from a schema governing a block, based on the cursor
//! context (key vs value position and the key path).

use crate::document::CursorPosition;
use serde_json::Value;

/// A completion candidate derived from a schema.
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    /// LSP `CompletionItemKind` (integer enum), e.g. 10 = Property.
    pub kind: i32,
    pub detail: Option<String>,
    /// Optional `insertText`. When present and containing snippet syntax
    /// (`${n}` tab stops), the caller must set `insertTextFormat` to Snippet (2).
    pub insert_text: Option<String>,
}

/// Returns completions for `schema` at the given `path`, depending on whether
/// the cursor is choosing a key or a value. `child_indent` is the indentation
/// (number of leading spaces) to use for lines inserted by a structure snippet.
pub fn complete(
    schema: &Value,
    path: &[String],
    position: CursorPosition,
    child_indent: usize,
) -> Vec<CompletionItem> {
    let Some(node) = schema_at(schema, path) else {
        return Vec::new();
    };
    match position {
        CursorPosition::Key => keys(node),
        CursorPosition::Value => values(node, child_indent),
    }
}

/// Walks `path` into `schema` via `properties` links.
fn schema_at<'a>(schema: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut node = schema;
    for key in path {
        node = node.get("properties")?.get(key)?;
    }
    Some(node)
}

/// Property-name completions for an object schema.
fn keys(schema: &Value) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let Some(props) = schema.get("properties").and_then(|p| p.as_object()) else {
        return items;
    };
    for (name, prop) in props {
        items.push(CompletionItem {
            label: name.clone(),
            kind: prop_kind(prop),
            detail: prop
                .get("description")
                .and_then(|d| d.as_str())
                .map(String::from),
            insert_text: None,
        });
    }
    items
}

/// Value completions for a scalar/collection schema: enums, consts, booleans,
/// a structure snippet for nested objects, empty arrays, and scalar placeholders.
fn values(schema: &Value, child_indent: usize) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    if let Some(const_val) = schema.get("const") {
        items.push(value_item(const_val));
        return items;
    }
    if let Some(enum_vals) = schema.get("enum").and_then(|e| e.as_array()) {
        for v in enum_vals {
            items.push(value_item(v));
        }
        return items;
    }
    match schema.get("type").and_then(|t| t.as_str()) {
        Some("boolean") => {
            items.push(value_item(&Value::Bool(true)));
            items.push(value_item(&Value::Bool(false)));
        }
        Some("object") => items.push(object_snippet(schema, child_indent)),
        Some("array") => items.push(CompletionItem {
            label: "[]".to_string(),
            kind: 11,
            detail: None,
            insert_text: None,
        }),
        Some("string") => items.push(CompletionItem {
            label: "\"\"".to_string(),
            kind: 10,
            detail: Some("string".to_string()),
            insert_text: None,
        }),
        Some("number") | Some("integer") => items.push(CompletionItem {
            label: "0".to_string(),
            kind: 12,
            detail: None,
            insert_text: None,
        }),
        _ => {}
    }
    items
}

/// Builds a snippet that seeds an object's structure: each property on its own
/// line at `child_indent`, with `$1..$n` tab stops for values and a final `$0`.
/// Required properties come first (falling back to all properties when the
/// schema declares none required).
fn object_snippet(schema: &Value, child_indent: usize) -> CompletionItem {
    let props = schema
        .get("properties")
        .and_then(|p| p.as_object())
        .cloned()
        .unwrap_or_default();
    let required: Vec<String> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|k| k.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let mut order = required.clone();
    for key in props.keys() {
        if !order.contains(key) {
            order.push(key.clone());
        }
    }

    let pad = " ".repeat(child_indent);
    let mut snippet = String::new();
    for (i, key) in order.iter().enumerate() {
        snippet.push_str(&format!("\n{pad}{key}: ${{{}}}", i + 1));
    }
    snippet.push_str(&format!("\n{pad}$0"));

    let label = if order.len() == 1 {
        order[0].clone()
    } else {
        format!("{}…", order.join(", "))
    };
    CompletionItem {
        label,
        kind: 9, // Module / structure
        detail: Some("structure".to_string()),
        insert_text: Some(snippet),
    }
}

fn value_item(v: &Value) -> CompletionItem {
    let label = match v {
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("\"{s}\""),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    };
    let kind = match v {
        Value::Bool(_) | Value::Null => 21, // Constant
        Value::Number(_) => 12,             // Value
        Value::String(_) => 10,             // Property
        _ => 12,                            // Value
    };
    CompletionItem {
        label,
        kind,
        detail: None,
        insert_text: None,
    }
}

fn prop_kind(prop: &Value) -> i32 {
    let t = prop
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("property");
    match t {
        "object" => 9,   // Module
        "array" => 11,   // Unit
        "boolean" => 21, // Constant
        "number" => 12,  // Value
        "integer" => 12, // Value
        "null" => 21,    // Constant
        "string" => 10,  // Property
        _ => 10,         // Property
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "enabled": {"type": "boolean", "description": "Whether enabled"},
                "version": {"type": "number"},
                "mode": {"type": "string", "enum": ["auto", "manual"]},
                "strict": {"const": true},
                "image": {"type": "object", "properties": {
                    "registry": {"type": "string"},
                    "tag": {"type": "string"}
                }},
                "labels": {"type": "array"}
            }
        })
    }

    #[test]
    fn key_position_lists_properties() {
        let items = complete(&schema(), &[], CursorPosition::Key, 0);
        let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();
        assert!(labels.contains(&"enabled".to_string()));
        assert!(labels.contains(&"version".to_string()));
    }

    #[test]
    fn value_position_suggests_boolean() {
        let items = complete(
            &schema(),
            &["enabled".to_string()],
            CursorPosition::Value,
            0,
        );
        let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();
        assert_eq!(labels, vec!["true".to_string(), "false".to_string()]);
    }

    #[test]
    fn value_position_suggests_enum() {
        let items = complete(&schema(), &["mode".to_string()], CursorPosition::Value, 0);
        let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();
        assert_eq!(
            labels,
            vec!["\"auto\"".to_string(), "\"manual\"".to_string()]
        );
    }

    #[test]
    fn value_position_suggests_const() {
        let items = complete(&schema(), &["strict".to_string()], CursorPosition::Value, 0);
        assert_eq!(items[0].label, "true");
    }

    #[test]
    fn value_position_suggests_nested_object_snippet() {
        // `image` is an object without `required`: the snippet seeds all its
        // properties, each on an indented line with tab-stop placeholders.
        let items = complete(&schema(), &["image".to_string()], CursorPosition::Value, 2);
        assert_eq!(items.len(), 1);
        let snip = items[0].insert_text.as_deref().expect("snippet");
        assert!(snip.contains("registry: ${1}"));
        assert!(snip.contains("tag: ${2}"));
        // The snippet starts with a newline and uses the child indent.
        assert!(snip.starts_with("\n  "));
    }

    #[test]
    fn value_position_object_snippet_puts_required_first() {
        let s = json!({
            "type": "object",
            "properties": {"tag": {"type": "string"}, "repository": {"type": "string"}},
            "required": ["repository"]
        });
        let items = complete(&s, &[], CursorPosition::Value, 2);
        let snip = items[0].insert_text.as_deref().unwrap();
        // Required property is seeded before the optional one.
        assert!(snip.find("repository: ${1}").unwrap() < snip.find("tag: ${2}").unwrap());
    }

    #[test]
    fn value_position_suggests_array() {
        let items = complete(&schema(), &["labels".to_string()], CursorPosition::Value, 0);
        assert_eq!(items[0].label, "[]");
    }

    #[test]
    fn value_position_suggests_scalar_placeholder() {
        let items = complete(
            &schema(),
            &["version".to_string()],
            CursorPosition::Value,
            0,
        );
        assert_eq!(items[0].label, "0");
    }
}
